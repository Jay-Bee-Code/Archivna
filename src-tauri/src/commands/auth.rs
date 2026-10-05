use crate::commands::documents::ArchiveError;
use crate::crypto;
use crate::db::{self, DbState, VaultKeyState};
use crate::sync::{store as sync_store, SyncState};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

/// أدوار النظام — تتحكم في *نوع العملية* المسموحة (قراءة/كتابة/إدارة)
pub const ROLE_ADMIN: &str = "admin";
pub const ROLE_ARCHIVIST: &str = "archivist";
pub const ROLE_REVIEWER: &str = "reviewer";
pub const ROLE_VIEWER: &str = "viewer";
pub const ALL_ROLES: &[&str] = &[ROLE_ADMIN, ROLE_ARCHIVIST, ROLE_REVIEWER, ROLE_VIEWER];

/// مستويات السرية — راجع migrations/0002_org_structure.sql
pub const CONF_NORMAL: i64 = 1;
// القيم 2 و3 (محدود التداول، سري) مرجعية للتوثيق والواجهة الأمامية فقط؛
// لا يستخدمها الكود صراحةً لأنها أرقام خام تُرسَل من الواجهة مباشرة
#[allow(dead_code)]
pub const CONF_LIMITED: i64 = 2;
#[allow(dead_code)]
pub const CONF_SECRET: i64 = 3;
pub const CONF_TOP_SECRET: i64 = 4;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserPublic {
    pub id: String,
    pub username: String,
    pub full_name: Option<String>,
    pub role: String,
    /// مستوى الاطّلاع (Clearance) — مستقل عن الدور: الدور يحكم نوع العملية،
    /// وهذا يحكم أعلى مستوى سرية وثيقة يمكن للمستخدم رؤيتها. المدير يتجاوزه دائمًا.
    pub clearance_level: i64,
    /// القسم — None تعني "موظف مركزي" يرى وثائق كل الأقسام (ضمن حدود تصريحه)
    pub department_id: Option<String>,
}

/// الجلسة الحالية — مستخدم واحد فقط في كل لحظة (تطبيق سطح مكتب أحادي المستخدم لكل جهاز)
pub struct SessionState(pub Mutex<Option<UserPublic>>);

#[derive(Debug, Serialize)]
pub struct UnlockResult {
    pub has_admin: bool,
}

fn app_data_dir(app_handle: &AppHandle) -> Result<std::path::PathBuf, ArchiveError> {
    app_handle
        .path()
        .app_data_dir()
        .map_err(|e| ArchiveError::Io(format!("تعذر تحديد مجلد بيانات التطبيق: {e}")))
}

/// هل توجد خزنة على هذا الجهاز أصلًا؟ (تحدد الواجهة: إعداد أولي أم فتح خزنة موجودة)
#[tauri::command]
pub fn vault_exists(app_handle: AppHandle) -> Result<bool, ArchiveError> {
    Ok(db::vault_exists(&app_data_dir(&app_handle)?))
}

/// فتح الخزنة بالعبارة السرية — ينشئها أول مرة، ويفتحها في المرات التالية.
/// عند النجاح: يفعّل اتصال قاعدة البيانات ومفتاح تشفير الملفات في ذاكرة التطبيق.
#[tauri::command]
pub fn unlock_vault(
    app_handle: AppHandle,
    db: State<DbState>,
    vault_key: State<VaultKeyState>,
    sync: State<SyncState>,
    passphrase: String,
) -> Result<UnlockResult, ArchiveError> {
    if passphrase.len() < 8 {
        return Err(ArchiveError::Invalid(
            "العبارة السرية يجب أن تكون 8 أحرف على الأقل".into(),
        ));
    }

    let dir = app_data_dir(&app_handle)?;
    let conn = db::open_encrypted_db(&dir, &passphrase)
        .map_err(|_| ArchiveError::Invalid("عبارة سرية خاطئة، أو قاعدة بيانات تالفة".into()))?;

    // تحميل مفتاح الخزنة الموجود، أو إنشاؤه وتغليفه إن كانت هذه أول مرة
    let existing: Option<(Vec<u8>, Vec<u8>, Vec<u8>)> = conn
        .query_row(
            "SELECT wrapped_key, wrap_salt, wrap_nonce FROM vault_config WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;

    let master_key: [u8; 32] = match existing {
        Some((wrapped, salt, nonce)) => {
            let wrap_key = crypto::derive_wrap_key(&passphrase, &salt);
            let plain = crypto::decrypt(&wrap_key, &wrapped, &nonce)
                .map_err(ArchiveError::Invalid)?;
            plain
                .try_into()
                .map_err(|_| ArchiveError::Invalid("مفتاح الخزنة تالف".into()))?
        }
        None => {
            let new_key = crypto::generate_vault_key();
            let salt = crypto::generate_salt(16);
            let wrap_key = crypto::derive_wrap_key(&passphrase, &salt);
            let (wrapped, nonce) = crypto::encrypt(&wrap_key, &new_key);
            conn.execute(
                "INSERT INTO vault_config (id, wrapped_key, wrap_salt, wrap_nonce) VALUES (1, ?1, ?2, ?3)",
                params![wrapped, salt, nonce.to_vec()],
            )?;
            new_key
        }
    };

    let has_admin: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;

    *vault_key.0.lock().unwrap() = Some(master_key);
    *db.0.lock().unwrap() = Some(conn);

    // إن كان مفتاح مزامنة المؤسسة محفوظًا على هذا الجهاز، شغّل المحرك تلقائيًا
    crate::commands::sync::start_engine_if_configured(
        &app_handle,
        db.0.clone(),
        vault_key.0.clone(),
        &sync,
    );

    Ok(UnlockResult {
        has_admin: has_admin > 0,
    })
}

/// إنشاء حساب المدير الأول — مسموح فقط إذا لم يوجد أي مستخدم بعد في هذه الخزنة.
/// المدير يُنشأ دائمًا بأعلى مستوى اطّلاع (4) — الدور نفسه يتجاوز فحص الاطّلاع أصلًا،
/// لكن تخزينه بشكل صريح أنظف للعرض والتدقيق.
#[tauri::command]
pub fn create_admin(
    db: State<DbState>,
    username: String,
    password: String,
    full_name: Option<String>,
) -> Result<UserPublic, ArchiveError> {
    let guard = db.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| ArchiveError::Invalid("الخزنة مقفلة".into()))?;

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
    if count > 0 {
        return Err(ArchiveError::Invalid("يوجد حساب مدير بالفعل في هذه الخزنة".into()));
    }
    if password.len() < 8 {
        return Err(ArchiveError::Invalid(
            "كلمة المرور يجب أن تكون 8 أحرف على الأقل".into(),
        ));
    }
    if username.trim().is_empty() {
        return Err(ArchiveError::Invalid("اسم المستخدم مطلوب".into()));
    }

    let id = Uuid::new_v4().to_string();
    let hash = crypto::hash_password(&password);
    let now = Utc::now().timestamp();

    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, full_name, created_at, clearance_level)
         VALUES (?1, ?2, ?3, 'admin', ?4, ?5, ?6)",
        params![id, username, hash, full_name, now, CONF_TOP_SECRET],
    )?;
    sync_store::log_local_event(
        conn,
        "user",
        &id,
        "upsert",
        &serde_json::json!({
            "id": id, "username": username, "password_hash": hash, "role": "admin",
            "full_name": full_name, "department_id": null, "clearance_level": CONF_TOP_SECRET,
            "created_at": now, "is_active": 1
        }),
    )?;

    Ok(UserPublic {
        id,
        username,
        full_name,
        role: ROLE_ADMIN.into(),
        clearance_level: CONF_TOP_SECRET,
        department_id: None,
    })
}

/// إنشاء مستخدم جديد بدور وتصريح اطّلاع وقسم محدَّدين — للمدير فقط
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_user(
    db: State<DbState>,
    session: State<SessionState>,
    sync: State<SyncState>,
    username: String,
    password: String,
    full_name: Option<String>,
    role: String,
    department_id: Option<String>,
    clearance_level: Option<i64>,
) -> Result<UserPublic, ArchiveError> {
    require_role(&session, &[ROLE_ADMIN])?;

    if ![ROLE_ADMIN, ROLE_ARCHIVIST, ROLE_REVIEWER, ROLE_VIEWER].contains(&role.as_str()) {
        return Err(ArchiveError::Invalid("دور غير معروف".into()));
    }
    if password.len() < 8 {
        return Err(ArchiveError::Invalid(
            "كلمة المرور يجب أن تكون 8 أحرف على الأقل".into(),
        ));
    }
    let clearance = clearance_level.unwrap_or(CONF_NORMAL).clamp(CONF_NORMAL, CONF_TOP_SECRET);

    let guard = db.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| ArchiveError::Invalid("الخزنة مقفلة".into()))?;

    let id = Uuid::new_v4().to_string();
    let hash = crypto::hash_password(&password);
    let now = Utc::now().timestamp();

    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, full_name, created_at, department_id, clearance_level)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, username, hash, role, full_name, now, department_id, clearance],
    ).map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            ArchiveError::Invalid("اسم المستخدم مستخدَم بالفعل".into())
        } else {
            ArchiveError::Db(e.to_string())
        }
    })?;

    sync_store::log_local_event(
        conn,
        "user",
        &id,
        "upsert",
        &serde_json::json!({
            "id": id, "username": username, "password_hash": hash, "role": role,
            "full_name": full_name, "department_id": department_id, "clearance_level": clearance,
            "created_at": now, "is_active": 1
        }),
    )?;
    sync.poke();

    Ok(UserPublic { id, username, full_name, role, clearance_level: clearance, department_id })
}

#[tauri::command]
pub fn login(
    db: State<DbState>,
    session: State<SessionState>,
    username: String,
    password: String,
) -> Result<UserPublic, ArchiveError> {
    let guard = db.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| ArchiveError::Invalid("الخزنة مقفلة".into()))?;

    let row: Option<(String, String, String, Option<String>, String, i64, i64, Option<String>)> = conn
        .query_row(
            "SELECT id, username, password_hash, full_name, role, is_active, clearance_level, department_id
             FROM users WHERE username = ?1",
            params![username],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?)),
        )
        .optional()?;

    let (id, username, hash, full_name, role, is_active, clearance_level, department_id) = row
        .ok_or_else(|| ArchiveError::Invalid("اسم مستخدم أو كلمة مرور خاطئة".into()))?;

    if is_active == 0 {
        return Err(ArchiveError::Invalid("هذا الحساب مُعطَّل — راجع المدير".into()));
    }
    if !crypto::verify_password(&password, &hash) {
        return Err(ArchiveError::Invalid("اسم مستخدم أو كلمة مرور خاطئة".into()));
    }

    let user = UserPublic { id, username, full_name, role, clearance_level, department_id };
    *session.0.lock().unwrap() = Some(user.clone());
    Ok(user)
}

/// قائمة المستخدمين النشطين — للمدير والأرشيفي فقط (تُستخدم لاختيار "مُحال إليه" عند الإحالة)
#[tauri::command]
pub fn list_users(db: State<DbState>, session: State<SessionState>) -> Result<Vec<UserPublic>, ArchiveError> {
    require_role(&session, &[ROLE_ADMIN, ROLE_ARCHIVIST])?;
    let guard = db.0.lock().unwrap();
    let conn = guard.as_ref().ok_or_else(|| ArchiveError::Invalid("الخزنة مقفلة".into()))?;

    let mut stmt = conn.prepare(
        "SELECT id, username, full_name, role, clearance_level, department_id
         FROM users WHERE is_active = 1 ORDER BY username ASC",
    )?;
    let users = stmt
        .query_map([], |r| {
            Ok(UserPublic {
                id: r.get(0)?, username: r.get(1)?, full_name: r.get(2)?, role: r.get(3)?,
                clearance_level: r.get(4)?, department_id: r.get(5)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();
    Ok(users)
}

#[tauri::command]
pub fn logout(session: State<SessionState>) -> Result<(), ArchiveError> {
    *session.0.lock().unwrap() = None;
    Ok(())
}

#[tauri::command]
pub fn current_user(session: State<SessionState>) -> Result<Option<UserPublic>, ArchiveError> {
    Ok(session.0.lock().unwrap().clone())
}

/// نقطة تحقق مركزية: يرفض العملية إن لم يكن هناك مستخدم مسجَّل دخوله،
/// أو كان دوره خارج القائمة المسموحة. يُستدعى في بداية كل أمر حساس.
pub fn require_role(
    session: &State<SessionState>,
    allowed: &[&str],
) -> Result<UserPublic, ArchiveError> {
    let guard = session.0.lock().unwrap();
    let user = guard
        .clone()
        .ok_or_else(|| ArchiveError::Invalid("يجب تسجيل الدخول أولًا".into()))?;
    if !allowed.contains(&user.role.as_str()) {
        return Err(ArchiveError::Invalid(
            "لا تملك صلاحية كافية لتنفيذ هذه العملية".into(),
        ));
    }
    Ok(user)
}

/// المدير (admin) يتجاوز فلترة القسم ومستوى الاطّلاع بالكامل ويرى كل شيء —
/// الاستعلامات في documents.rs تتحقق من هذا وتبني شرط SQL إضافيًا لغيره:
/// confidentiality_level <= <تصريحه> AND (<قسمه> IS NULL OR department_id IS NULL OR department_id = <قسمه>)
pub fn bypasses_visibility_filter(role: &str) -> bool {
    role == ROLE_ADMIN
}

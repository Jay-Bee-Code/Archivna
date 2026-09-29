use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::password_hash::{rand_core::OsRng as PwOsRng, PasswordHash, SaltString};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use rand::RngCore;

pub const NONCE_LEN: usize = 12;

/// يشتق مفتاح تغليف/تشفير 32 بايت من عبارة سرية + ملح (salt) عبر Argon2id.
/// يُستخدم هذا لتغليف مفتاح الخزنة (Vault Master Key) — وليس لتشفير الملفات مباشرة،
/// حتى يمكن تغيير العبارة السرية لاحقًا دون إعادة تشفير كامل الأرشيف (Key Wrapping).
pub fn derive_wrap_key(passphrase: &str, salt: &[u8]) -> [u8; 32] {
    let argon2 = Argon2::default();
    let mut out = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut out)
        .expect("فشل اشتقاق مفتاح التغليف عبر Argon2");
    out
}

/// تشفير AES-256-GCM عام (مصادَق — أي تلاعب بالبيانات المشفّرة يُكتشف عند فك التشفير)
/// يعيد (النص المشفّر, الـ Nonce العشوائي المستخدم)
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> (Vec<u8>, [u8; NONCE_LEN]) {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .expect("فشل التشفير — لا يُفترض حدوثه مع AES-GCM");
    (ciphertext, nonce_bytes)
}

pub fn decrypt(key: &[u8; 32], ciphertext: &[u8], nonce_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "فشل فك التشفير — العبارة السرية أو المفتاح غير صحيح، أو البيانات تالفة".to_string())
}

/// توليد مفتاح خزنة عشوائي جديد (32 بايت) — يُستخدم مرة واحدة فقط عند أول تشغيل،
/// ثم يُغلَّف ويُخزَّن في جدول vault_config
pub fn generate_vault_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

pub fn generate_salt(len: usize) -> Vec<u8> {
    let mut salt = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

/// تجزئة كلمة مرور مستخدم (Argon2id + ملح عشوائي مُضمَّن في السلسلة الناتجة)
/// لتخزينها في عمود password_hash — التنسيق القياسي PHC يسمح بالتحقق لاحقًا بدون تخزين الملح منفصلًا
pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut PwOsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("فشل تجزئة كلمة المرور")
        .to_string()
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

/// مفتاح مزامنة المؤسسة: يُشتق بشكل حتمي من عبارة سرية مشتركة (ملح ثابت للتطبيق)
/// حتى تصل كل الأجهزة التي تعرف العبارة إلى نفس المفتاح دون تبادل أي شيء.
pub fn derive_sync_key(passphrase: &str) -> [u8; 32] {
    derive_wrap_key(passphrase, b"govt-archive-sync-v1")
}

pub fn random_hex(n_bytes: usize) -> String {
    let mut b = vec![0u8; n_bytes];
    rand::thread_rng().fill_bytes(&mut b);
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

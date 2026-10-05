// يمنع فتح نافذة Console إضافية على ويندوز في وضع الإنتاج
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod crypto;
mod db;
mod ocr;
mod qr;
mod sync;

use commands::auth::SessionState;
use db::{DbState, VaultKeyState};
use std::sync::{Arc, Mutex};
use sync::SyncState;

fn main() {
    tauri::Builder::default()
        .manage(DbState(Arc::new(Mutex::new(None))))
        .manage(VaultKeyState(Arc::new(Mutex::new(None))))
        .manage(SessionState(Mutex::new(None)))
        .manage(SyncState(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            // المصادقة وفتح الخزنة
            commands::auth::vault_exists,
            commands::auth::unlock_vault,
            commands::auth::create_admin,
            commands::auth::create_user,
            commands::auth::login,
            commands::auth::logout,
            commands::auth::current_user,
            commands::auth::list_users,
            // الوثائق
            commands::documents::add_document,
            commands::documents::list_documents,
            commands::documents::search_documents,
            commands::documents::get_document_file,
            commands::documents::delete_document,
            commands::documents::ocr_status,
            commands::documents::update_document_status,
            // الفئات
            commands::categories::add_category,
            commands::categories::list_categories,
            commands::categories::delete_category,
            commands::categories::list_documents_by_category,
            // الأقسام (الهيكل التنظيمي)
            commands::departments::add_department,
            commands::departments::list_departments,
            commands::departments::delete_department,
            // أنواع الوثائق
            commands::document_types::add_document_type,
            commands::document_types::list_document_types,
            commands::document_types::delete_document_type,
            commands::document_types::suggested_document_types,
            // الاحتفاظ والإتلاف
            commands::retention::set_document_type_retention,
            commands::retention::set_legal_hold,
            commands::retention::list_disposal_candidates,
            commands::retention::dispose_document,
            commands::retention::list_disposal_log,
            // الأرشيف الورقي
            commands::physical::update_physical,
            commands::physical::get_document_qr,
            // جهات المراسلة
            commands::correspondents::add_correspondent,
            commands::correspondents::list_correspondents,
            commands::correspondents::delete_correspondent,
            // سجل الإحالة
            commands::routing::route_document,
            commands::routing::list_document_routing,
            // المزامنة
            commands::sync::sync_status,
            commands::sync::has_users,
            commands::sync::set_sync_key,
            commands::sync::sync_now,
        ])
        .run(tauri::generate_context!())
        .expect("خطأ أثناء تشغيل تطبيق Tauri");
}

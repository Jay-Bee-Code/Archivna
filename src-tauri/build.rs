fn main() {
    // Tauri يُولِّد تعليمات البناء (أيقونات Windows، الصلاحيات، schemas) للتطبيق فقط.
    // نستخدم cfg وقت التجميع (لا if وقت التشغيل): Rust يُجمِّع الفرع كله حتى لو لم يُنفَّذ،
    // فيفشل التجميع إن كان tauri_build غير متاح. cfg(feature) تعمل داخل build.rs (تحقّقنا).
    #[cfg(feature = "desktop")]
    tauri_build::build();
}

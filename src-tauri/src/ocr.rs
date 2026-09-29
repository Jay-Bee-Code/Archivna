//! استخراج نص من الصور عبر Tesseract OCR.
//!
//! قرار هندسي متعمَّد: نستدعي الملف التنفيذي `tesseract` كعملية فرعية (subprocess)
//! بدل الربط الثابت عبر FFI (crates مثل leptess/tesseract تحتاج libtesseract +
//! libleptonica عند البناء — تبعية native إضافية على كل جهاز تطوير، وقد تتكرر معها
//! نفس متاعب OpenSSL/Perl التي واجهناها في Phase 3). النتيجة: لا تغيير على خطوات
//! البناء إطلاقًا، والتطبيق يعمل بلا OCR إن لم يكن Tesseract مثبَّتًا (تدهور رشيق).

use std::io::Write;
use std::process::{Command, Stdio};

/// هل Tesseract مثبَّت ومتاح في PATH على هذا الجهاز؟
pub fn is_available() -> bool {
    Command::new("tesseract")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// يستخرج النص من صورة (jpg/png/tiff/bmp). يُرجع None بصمت إن لم يكن Tesseract
/// مثبَّتًا، أو لنوع ملف غير مدعوم، أو إن لم يُستخرج نص — الإضافة تتابع دون OCR،
/// وهذا ليس خطأً يستحق مقاطعة المستخدم.
///
/// نطاق هذه المرحلة: الصور فقط. OCR لملفات PDF يتطلب تفريغها لصور أولًا (عادة عبر
/// poppler/pdftoppm)، وهذا مؤجَّل عمدًا — راجع SETUP.md.
pub fn extract_text(bytes: &[u8], mime_type: Option<&str>) -> Option<String> {
    let mime = mime_type?;
    if !mime.starts_with("image/") {
        return None;
    }
    if bytes.is_empty() {
        return None;
    }

    // نُمرِّر البيانات عبر stdin بدل ملف مؤقت — لا كتابة صافية (غير مشفَّرة) على القرص أبدًا
    let run = |lang: Option<&str>| -> Option<Vec<u8>> {
        let mut cmd = Command::new("tesseract");
        cmd.arg("-").arg("stdout").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
        if let Some(l) = lang {
            cmd.arg("-l").arg(l);
        }
        let mut child = cmd.spawn().ok()?;
        child.stdin.take()?.write_all(bytes).ok()?;
        let output = child.wait_with_output().ok()?;
        output.status.success().then_some(output.stdout)
    };

    // نحاول عربي+إنجليزي أولًا؛ إن لم تكن حزمة "ara" مثبَّتة نتراجع للافتراضي (eng)
    let stdout = run(Some("ara+eng")).or_else(|| run(None))?;
    let text = String::from_utf8_lossy(&stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

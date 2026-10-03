//! توليد باركود QR لطباعته على غلاف الملف الورقي — يفتح الوثيقة الرقمية عند مسحه.
//!
//! قرار هندسي بنفس منطق OCR (راجع ocr.rs): مكتبة `qrcode` بحتة Rust بلا أي
//! تبعية بناء native، تُخرج SVG نصيًا مباشرة — لا PNG ولا مكتبة صور إضافية،
//! فلا خطر بناء جديد إطلاقًا (لا Perl، لا OpenSSL، لا libtesseract).

use qrcode::render::svg;
use qrcode::QrCode;

/// يولّد SVG لرمز QR يحمل النص المعطى (عادة رقم القيد، أو معرّف الوثيقة كبديل)
pub fn generate_svg(data: &str) -> Result<String, String> {
    let code = QrCode::new(data.as_bytes()).map_err(|e| format!("فشل توليد رمز QR: {e}"))?;
    let svg = code
        .render::<svg::Color>()
        .min_dimensions(240, 240)
        .dark_color(svg::Color("#1A3A5C"))
        .light_color(svg::Color("#FFFFFF"))
        .build();
    Ok(svg)
}

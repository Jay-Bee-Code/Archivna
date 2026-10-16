import { useEffect } from "react";

/** إغلاق النافذة/اللوحة المنبثقة بمفتاح Esc — سلوك متوقَّع من أي تطبيق مكتبي احترافي */
export function useEscape(onClose: () => void) {
  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, [onClose]);
}

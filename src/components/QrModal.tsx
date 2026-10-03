import { useEffect, useState } from "react";
import { api, errorMessage } from "../lib/api";

export default function QrModal({
  documentId,
  title,
  onClose,
}: {
  documentId: string;
  title: string;
  onClose: () => void;
}) {
  const [svg, setSvg] = useState<string | null>(null);
  const [payload, setPayload] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .getDocumentQr(documentId)
      .then((r) => {
        setSvg(r.svg);
        setPayload(r.payload);
      })
      .catch((e) => setError(errorMessage(e)));
  }, [documentId]);

  function handlePrint() {
    const win = window.open("", "_blank");
    if (!win) return;
    win.document.write(`
      <html dir="rtl"><head><title>طباعة ملصق — ${title}</title></head>
      <body style="font-family: sans-serif; text-align:center; padding:24px;">
        <div>${svg}</div>
        <p style="font-size:14px; margin-top:8px;">${title}</p>
        <p style="font-size:12px; color:#666; font-family:monospace;">${payload}</p>
        <script>window.onload = () => window.print();</script>
      </body></html>
    `);
    win.document.close();
  }

  return (
    <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-50 px-4" onClick={onClose}>
      <div
        onClick={(e) => e.stopPropagation()}
        className="w-full max-w-xs bg-white dark:bg-[#132033] border border-border-light dark:border-white/10 rounded-2xl p-6 flex flex-col items-center gap-3 text-center"
      >
        <h2 className="text-sm font-bold text-primary dark:text-white truncate w-full">{title}</h2>

        {error && <p className="text-sm text-danger">{error}</p>}
        {!svg && !error && <p className="text-xs text-text-secondary">جارٍ التوليد…</p>}
        {svg && (
          <div
            className="bg-white p-3 rounded-lg"
            dangerouslySetInnerHTML={{ __html: svg }}
          />
        )}
        {payload && <p className="text-[11px] text-text-secondary dark:text-white/40 font-mono break-all">{payload}</p>}

        <p className="text-[11px] text-text-secondary dark:text-white/40">
          اطبع هذا الرمز على غلاف الملف الورقي لربطه بالنسخة الرقمية
        </p>

        <div className="flex gap-2 w-full mt-1">
          <button
            onClick={onClose}
            className="flex-1 py-2 rounded-lg border border-border-light dark:border-white/10 text-text-secondary dark:text-white/60 text-sm hover:bg-black/5 dark:hover:bg-white/5"
          >
            إغلاق
          </button>
          <button
            onClick={handlePrint}
            disabled={!svg}
            className="flex-1 py-2 rounded-lg bg-primary text-white text-sm hover:bg-accent disabled:opacity-40"
          >
            طباعة
          </button>
        </div>
      </div>
    </div>
  );
}

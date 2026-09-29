import { useState } from "react";
import { Document, api, downloadBase64, errorMessage } from "../lib/api";

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} بايت`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} كيلوبايت`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} ميغابايت`;
}

function formatDate(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleDateString("ar-DZ", {
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}

export default function DocumentCard({
  doc,
  canWrite,
  onDelete,
}: {
  doc: Document;
  canWrite: boolean;
  onDelete: (id: string) => void;
}) {
  const [downloading, setDownloading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleDownload() {
    setDownloading(true);
    setError(null);
    try {
      const base64 = await api.getDocumentFile(doc.id);
      downloadBase64(base64, doc.title, doc.mime_type);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setDownloading(false);
    }
  }

  return (
    <div className="border border-border-light dark:border-white/10 rounded-lg p-4 bg-white dark:bg-white/5 hover:shadow-md transition-shadow flex flex-col gap-2">
      <div className="flex items-start justify-between gap-2">
        <h3 className="font-semibold text-text-primary dark:text-white truncate">{doc.title}</h3>
        {canWrite && (
          <button
            onClick={() => onDelete(doc.id)}
            className="text-danger/70 hover:text-danger text-sm shrink-0"
            title="حذف"
          >
            حذف
          </button>
        )}
      </div>

      <div className="flex flex-wrap gap-x-3 text-xs text-text-secondary dark:text-white/50">
        <span>{formatSize(doc.file_size)}</span>
        <span>•</span>
        <span>{formatDate(doc.created_at)}</span>
        {doc.mime_type && (
          <>
            <span>•</span>
            <span>{doc.mime_type}</span>
          </>
        )}
      </div>

      <div className="flex items-center justify-between mt-1">
        <div className="flex items-center gap-1.5">
          <span className="text-[11px] px-2 py-0.5 rounded-full bg-success/10 text-success">
            🔒 مشفّر
          </span>
          {doc.has_ocr && (
            <span
              className="text-[11px] px-2 py-0.5 rounded-full bg-accent/10 text-accent"
              title="النص داخل هذه الصورة قابل للبحث (OCR)"
            >
              📝 نص مستخرَج
            </span>
          )}
        </div>
        <button
          onClick={handleDownload}
          disabled={downloading}
          className="text-xs text-accent hover:underline disabled:opacity-50"
        >
          {downloading ? "جارٍ فك التشفير…" : "تنزيل"}
        </button>
      </div>

      {error && <p className="text-xs text-danger">{error}</p>}
    </div>
  );
}

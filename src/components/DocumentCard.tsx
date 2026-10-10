import { useState } from "react";
import { Document, api, downloadBase64, errorMessage, CONFIDENTIALITY_LABELS, STATUS_LABELS, PHYSICAL_STATUS_LABELS } from "../lib/api";
import QrModal from "./QrModal";
import RoutingModal from "./RoutingModal";
import { IconLock, IconScale, IconBox, IconNote, IconRoute, IconQr, IconDownload, IconSpinner, IconTrash } from "./icons/Icon";

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} بايت`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} كيلوبايت`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} ميغابايت`;
}

const CONF_STYLES: Record<number, string> = {
  1: "bg-black/5 dark:bg-white/5 text-text-secondary dark:text-white/50",
  2: "bg-warning/10 text-warning",
  3: "bg-danger/10 text-danger",
  4: "bg-danger/15 text-danger font-semibold",
};

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
  onOpen,
}: {
  doc: Document;
  canWrite: boolean;
  onDelete: (id: string) => void;
  onOpen: () => void;
}) {
  const [downloading, setDownloading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showQr, setShowQr] = useState(false);
  const [showRouting, setShowRouting] = useState(false);

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
    <div className="group border border-border-light dark:border-white/10 rounded-xl p-4 bg-white dark:bg-white/[0.03] hover:border-accent/40 hover:shadow-md dark:hover:shadow-black/20 transition-all flex flex-col gap-2.5">
      <div className="flex items-start justify-between gap-2">
        <button
          type="button"
          onClick={onOpen}
          title="عرض التفاصيل"
          className="group/title flex items-center gap-2 min-w-0 text-start cursor-pointer"
        >
          <span className="shrink-0 w-8 h-8 rounded-lg bg-primary/10 dark:bg-accent/15 flex items-center justify-center text-primary dark:text-accent text-[10px] font-bold uppercase">
            {(() => {
              const ext = doc.title.includes(".") ? doc.title.split(".").pop() : null;
              const sub = doc.mime_type?.split("/")[1]?.replace("jpeg", "jpg");
              return (ext || sub || "؟").slice(0, 4).toUpperCase();
            })()}
          </span>
          <div className="min-w-0">
            <h3 className="font-semibold text-text-primary dark:text-white truncate group-hover/title:text-accent transition-colors">{doc.title}</h3>
            {doc.registry_number && (
              <p className="text-[11px] text-text-secondary dark:text-white/35 font-mono truncate">
                {doc.registry_number}
              </p>
            )}
          </div>
        </button>
        {canWrite && (
          <button
            onClick={() => onDelete(doc.id)}
            className="flex items-center gap-1 text-text-secondary/50 hover:text-danger dark:text-white/30 dark:hover:text-danger text-xs shrink-0 opacity-0 group-hover:opacity-100 transition-opacity"
            title="حذف"
          >
            <IconTrash size={13} />
          </button>
        )}
      </div>

      <div className="flex flex-wrap gap-x-3 text-xs text-text-secondary dark:text-white/40">
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
        <div className="flex items-center gap-1.5 flex-wrap">
          <span className="flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-success/10 text-success">
            <IconLock size={11} /> مشفّر
          </span>
          {doc.confidentiality_level > 1 && (
            <span className={`text-[11px] px-2 py-0.5 rounded-full ${CONF_STYLES[doc.confidentiality_level]}`}>
              {CONFIDENTIALITY_LABELS[doc.confidentiality_level]}
            </span>
          )}
          {doc.status !== "approved" && (
            <span className="text-[11px] px-2 py-0.5 rounded-full bg-accent/10 text-accent">
              {STATUS_LABELS[doc.status] || doc.status}
            </span>
          )}
          {doc.legal_hold && (
            <span
              className="flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-warning/15 text-warning"
              title="مجمَّدة قانونيًا — لا يمكن إتلافها"
            >
              <IconScale size={11} /> مجمَّدة
            </span>
          )}
          {doc.physical_status !== "none" && (
            <span className="flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-black/5 dark:bg-white/5 text-text-secondary dark:text-white/60">
              <IconBox size={11} /> {PHYSICAL_STATUS_LABELS[doc.physical_status] || doc.physical_status}
            </span>
          )}
          {doc.has_ocr && (
            <span
              className="flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-accent/10 text-accent"
              title="النص داخل هذه الصورة قابل للبحث (OCR)"
            >
              <IconNote size={11} /> نص مستخرَج
            </span>
          )}
        </div>
        <div className="flex items-center gap-3">
          {canWrite && (
            <button
              onClick={() => setShowRouting(true)}
              className="flex items-center gap-1 text-xs text-text-secondary dark:text-white/50 hover:text-accent transition-colors"
            >
              <IconRoute size={13} /> إحالة
            </button>
          )}
          <button
            onClick={() => setShowQr(true)}
            className="flex items-center gap-1 text-xs text-text-secondary dark:text-white/50 hover:text-accent transition-colors"
          >
            <IconQr size={13} /> QR
          </button>
          <button
            onClick={handleDownload}
            disabled={downloading}
            className="flex items-center gap-1 text-xs text-accent hover:text-primary dark:hover:text-white font-medium disabled:opacity-50 transition-colors"
          >
            {downloading ? (
              <>
                <IconSpinner size={12} /> جارٍ فك التشفير…
              </>
            ) : (
              <>
                <IconDownload size={13} /> تنزيل
              </>
            )}
          </button>
        </div>
      </div>

      {error && <p className="text-xs text-danger">{error}</p>}
      {showQr && <QrModal documentId={doc.id} title={doc.title} onClose={() => setShowQr(false)} />}
      {showRouting && <RoutingModal documentId={doc.id} title={doc.title} onClose={() => setShowRouting(false)} />}
    </div>
  );
}

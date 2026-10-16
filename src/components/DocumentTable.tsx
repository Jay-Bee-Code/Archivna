import { useMemo, useState } from "react";
import {
  api,
  Document,
  Department,
  DocumentType,
  downloadBase64,
  errorMessage,
  CONFIDENTIALITY_LABELS,
  STATUS_LABELS,
  PHYSICAL_STATUS_LABELS,
} from "../lib/api";
import Bidi from "./Bidi";
import QrModal from "./QrModal";
import RoutingModal from "./RoutingModal";
import { IconScale, IconBox, IconNote, IconRoute, IconQr, IconDownload, IconSpinner, IconTrash } from "./icons/Icon";

type SortKey = "title" | "created_at" | "confidentiality_level";

const CONF_CHIP: Record<number, string> = {
  2: "bg-warning/10 text-warning",
  3: "bg-danger/10 text-danger",
  4: "bg-danger/15 text-danger font-semibold",
};

const STATUS_DOT: Record<string, string> = {
  approved: "bg-success",
  draft: "bg-text-secondary/50 dark:bg-white/30",
  in_review: "bg-warning",
  archived: "bg-accent",
  superseded: "bg-text-secondary/50 dark:bg-white/30",
  disposed: "bg-danger",
};

function formatDate(ts: number): string {
  return new Date(ts * 1000).toLocaleDateString("ar-DZ", { year: "numeric", month: "short", day: "numeric" });
}

function SortHeader({
  label,
  k,
  sort,
  onSort,
  className = "",
}: {
  label: string;
  k: SortKey;
  sort: { key: SortKey; dir: 1 | -1 };
  onSort: (k: SortKey) => void;
  className?: string;
}) {
  const active = sort.key === k;
  return (
    <th className={`px-4 py-2.5 text-start font-medium ${className}`}>
      <button
        onClick={() => onSort(k)}
        className={`inline-flex items-center gap-1 hover:text-text-primary dark:hover:text-white transition-colors ${
          active ? "text-text-primary dark:text-white" : ""
        }`}
      >
        {label}
        <span className={`text-[10px] ${active ? "opacity-100" : "opacity-0"}`}>{sort.dir === 1 ? "▲" : "▼"}</span>
      </button>
    </th>
  );
}

function Row({
  doc,
  canWrite,
  typeName,
  deptName,
  onOpen,
  onDelete,
}: {
  doc: Document;
  canWrite: boolean;
  typeName: string | null;
  deptName: string | null;
  onOpen: () => void;
  onDelete: () => void;
}) {
  const [downloading, setDownloading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showQr, setShowQr] = useState(false);
  const [showRouting, setShowRouting] = useState(false);

  async function handleDownload() {
    setDownloading(true);
    setError(null);
    try {
      downloadBase64(await api.getDocumentFile(doc.id), doc.title, doc.mime_type);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setDownloading(false);
    }
  }

  const iconBtn =
    "p-1.5 rounded-md text-text-secondary dark:text-white/50 hover:text-accent hover:bg-accent/10 transition-colors";

  return (
    <tr className="group border-t border-border-light/70 dark:border-white/5 hover:bg-black/[0.02] dark:hover:bg-white/[0.03] transition-colors">
      <td className="px-4 py-3 w-full min-w-[14rem]">
        <button
          type="button"
          onClick={onOpen}
          data-doc-open
          title="عرض التفاصيل"
          className="block w-full text-start cursor-pointer"
        >
          <span className="flex items-center gap-1.5 min-w-0">
            <span className="font-medium text-text-primary dark:text-white group-hover:text-accent transition-colors line-clamp-2 min-w-0">
              <Bidi>{doc.title}</Bidi>
            </span>
            {doc.legal_hold && (
              <span title="مجمَّدة قانونيًا — لا يمكن إتلافها" className="shrink-0 text-warning">
                <IconScale size={13} />
              </span>
            )}
            {doc.physical_status !== "none" && (
              <span
                title={`الأصل الورقي: ${PHYSICAL_STATUS_LABELS[doc.physical_status] || doc.physical_status}`}
                className="shrink-0 text-text-secondary dark:text-white/40"
              >
                <IconBox size={13} />
              </span>
            )}
            {doc.has_ocr && (
              <span title="النص داخل الصورة قابل للبحث (OCR)" className="shrink-0 text-accent">
                <IconNote size={13} />
              </span>
            )}
          </span>
          {doc.registry_number && (
            <span className="block mt-0.5 text-[11px] font-mono text-text-secondary dark:text-white/40 text-start">
              <Bidi>{doc.registry_number}</Bidi>
            </span>
          )}
        </button>
        {error && <p className="text-xs text-danger mt-1">{error}</p>}
      </td>

      <td className="px-4 py-3 text-text-secondary dark:text-white/60 hidden xl:table-cell whitespace-nowrap">
        {typeName ?? "—"}
      </td>
      <td className="px-4 py-3 text-text-secondary dark:text-white/60 hidden 2xl:table-cell max-w-[10rem] truncate">
        {deptName ?? "—"}
      </td>

      <td className="px-4 py-3 whitespace-nowrap">
        {doc.confidentiality_level > 1 ? (
          <span className={`text-[11px] px-2 py-0.5 rounded-full ${CONF_CHIP[doc.confidentiality_level]}`}>
            {CONFIDENTIALITY_LABELS[doc.confidentiality_level]}
          </span>
        ) : (
          <span className="text-xs text-text-secondary/70 dark:text-white/35">
            {CONFIDENTIALITY_LABELS[1]}
          </span>
        )}
      </td>

      <td className="px-4 py-3 whitespace-nowrap">
        <span className="inline-flex items-center gap-1.5 text-xs text-text-secondary dark:text-white/60">
          <span className={`w-1.5 h-1.5 rounded-full ${STATUS_DOT[doc.status] ?? "bg-text-secondary/50"}`} />
          {STATUS_LABELS[doc.status] || doc.status}
        </span>
      </td>

      <td className="px-4 py-3 text-xs text-text-secondary dark:text-white/50 whitespace-nowrap hidden lg:table-cell">
        {formatDate(doc.created_at)}
      </td>

      <td className="px-3 py-3 whitespace-nowrap">
        <div className="flex items-center justify-end gap-0.5">
          <button onClick={handleDownload} disabled={downloading} title="تنزيل (يُفَك تشفيره)" className={iconBtn}>
            {downloading ? <IconSpinner size={15} /> : <IconDownload size={15} />}
          </button>
          {canWrite && (
            <button onClick={() => setShowRouting(true)} title="إحالة إلى زميل" className={iconBtn}>
              <IconRoute size={15} />
            </button>
          )}
          <button onClick={() => setShowQr(true)} title="رمز QR للملف الورقي" className={iconBtn}>
            <IconQr size={15} />
          </button>
          {canWrite && (
            <button
              onClick={onDelete}
              title="حذف"
              className="p-1.5 rounded-md text-text-secondary dark:text-white/50 hover:text-danger hover:bg-danger/10 transition-colors"
            >
              <IconTrash size={15} />
            </button>
          )}
        </div>
        {showQr && <QrModal documentId={doc.id} title={doc.title} onClose={() => setShowQr(false)} />}
        {showRouting && <RoutingModal documentId={doc.id} title={doc.title} onClose={() => setShowRouting(false)} />}
      </td>
    </tr>
  );
}

export default function DocumentTable({
  docs,
  canWrite,
  types,
  departments,
  onOpen,
  onDelete,
}: {
  docs: Document[];
  canWrite: boolean;
  types: DocumentType[];
  departments: Department[];
  onOpen: (id: string) => void;
  onDelete: (id: string) => void;
}) {
  const [sort, setSort] = useState<{ key: SortKey; dir: 1 | -1 }>({ key: "created_at", dir: -1 });

  function onSort(k: SortKey) {
    setSort((s) => (s.key === k ? { key: k, dir: (s.dir * -1) as 1 | -1 } : { key: k, dir: k === "title" ? 1 : -1 }));
  }

  const sorted = useMemo(() => {
    const arr = [...docs];
    arr.sort((a, b) => {
      const va = a[sort.key];
      const vb = b[sort.key];
      const c = typeof va === "string" ? va.localeCompare(vb as string, "ar") : (va as number) - (vb as number);
      return c * sort.dir;
    });
    return arr;
  }, [docs, sort]);

  return (
    <div className="bg-white dark:bg-white/[0.02] border border-border-light dark:border-white/10 rounded-xl overflow-hidden">
      <table className="w-full text-sm">
        <thead className="bg-black/[0.03] dark:bg-white/[0.04] text-xs text-text-secondary dark:text-white/50">
          <tr>
            <SortHeader label="العنوان ورقم القيد" k="title" sort={sort} onSort={onSort} />
            <th className="px-4 py-2.5 text-start font-medium hidden xl:table-cell">النوع</th>
            <th className="px-4 py-2.5 text-start font-medium hidden 2xl:table-cell">القسم</th>
            <SortHeader label="السرية" k="confidentiality_level" sort={sort} onSort={onSort} />
            <th className="px-4 py-2.5 text-start font-medium">الحالة</th>
            <SortHeader label="التاريخ" k="created_at" sort={sort} onSort={onSort} className="hidden lg:table-cell" />
            <th className="px-3 py-2.5 w-px" />
          </tr>
        </thead>
        <tbody>
          {sorted.map((d) => (
            <Row
              key={d.id}
              doc={d}
              canWrite={canWrite}
              typeName={types.find((t) => t.id === d.document_type_id)?.name ?? null}
              deptName={departments.find((x) => x.id === d.department_id)?.name_ar ?? null}
              onOpen={() => onOpen(d.id)}
              onDelete={() => onDelete(d.id)}
            />
          ))}
        </tbody>
      </table>
    </div>
  );
}

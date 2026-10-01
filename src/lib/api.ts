import { invoke } from "@tauri-apps/api/core";

export interface Document {
  id: string;
  title: string;
  category_id: string | null;
  file_hash: string;
  file_size: number;
  mime_type: string | null;
  created_at: number;
  updated_at: number | null;
  has_ocr: boolean;
  department_id: string | null;
  document_type_id: string | null;
  registry_number: string | null;
  confidentiality_level: number;
  status: string;
}

export interface NewDocumentInput {
  title: string;
  category_id: string | null;
  file_base64: string;
  mime_type: string | null;
  departmentId: string | null;
  documentTypeId: string | null;
  confidentialityLevel: number | null;
}

export interface Department {
  id: string;
  name_ar: string;
  name_fr: string | null;
  code: string | null;
  parent_id: string | null;
  head_user_id: string | null;
  is_active: boolean;
  created_at: number;
}

export interface NewDepartmentInput {
  name_ar: string;
  name_fr: string | null;
  code: string | null;
  parent_id: string | null;
}

export interface DocumentType {
  id: string;
  name: string;
  created_at: number;
}

export const CONFIDENTIALITY_LABELS: Record<number, string> = {
  1: "عادي",
  2: "محدود التداول",
  3: "سري",
  4: "سري جدًا",
};

export const STATUS_LABELS: Record<string, string> = {
  draft: "مسودة",
  in_review: "قيد المراجعة",
  approved: "معتمدة",
  archived: "مؤرشفة",
  superseded: "ملغاة/مستبدلة",
};

export interface Category {
  id: string;
  name: string;
  parent_id: string | null;
  created_at: number;
  document_count: number;
}

export interface NewCategoryInput {
  name: string;
  parent_id: string | null;
}

export type Role = "admin" | "archivist" | "reviewer" | "viewer";

export interface PeerView {
  node_id: string;
  name: string;
  addr: string;
  last_seen: number;
  last_sync: number | null;
  last_error: string | null;
}

export interface SyncStatus {
  configured: boolean;
  running: boolean;
  node_id: string | null;
  node_name: string | null;
  peers: PeerView[];
  last_sync: number | null;
}

export interface UserPublic {
  id: string;
  username: string;
  full_name: string | null;
  role: Role;
  clearance_level: number;
  department_id: string | null;
}

export const api = {
  // المصادقة وفتح الخزنة
  vaultExists: () => invoke<boolean>("vault_exists"),
  unlockVault: (passphrase: string) =>
    invoke<{ has_admin: boolean }>("unlock_vault", { passphrase }),
  createAdmin: (username: string, password: string, fullName: string | null) =>
    invoke<UserPublic>("create_admin", { username, password, fullName }),
  createUser: (input: {
    username: string;
    password: string;
    fullName: string | null;
    role: Role;
    departmentId: string | null;
    clearanceLevel: number | null;
  }) => invoke<UserPublic>("create_user", input),
  login: (username: string, password: string) =>
    invoke<UserPublic>("login", { username, password }),
  logout: () => invoke<void>("logout"),
  currentUser: () => invoke<UserPublic | null>("current_user"),

  // الوثائق
  addDocument: (input: NewDocumentInput) => invoke<Document>("add_document", { input }),
  listDocuments: () => invoke<Document[]>("list_documents"),
  searchDocuments: (query: string) => invoke<Document[]>("search_documents", { query }),
  getDocumentFile: (id: string) => invoke<string>("get_document_file", { id }),
  deleteDocument: (id: string) => invoke<void>("delete_document", { id }),

  // الفئات
  addCategory: (input: NewCategoryInput) => invoke<Category>("add_category", { input }),
  listCategories: () => invoke<Category[]>("list_categories"),
  deleteCategory: (id: string) => invoke<void>("delete_category", { id }),
  listDocumentsByCategory: (categoryId: string) =>
    invoke<Document[]>("list_documents_by_category", { categoryId }),

  // OCR
  ocrStatus: () => invoke<boolean>("ocr_status"),

  // الأقسام
  addDepartment: (input: NewDepartmentInput) => invoke<Department>("add_department", { input }),
  listDepartments: () => invoke<Department[]>("list_departments"),
  deleteDepartment: (id: string) => invoke<void>("delete_department", { id }),

  // أنواع الوثائق
  addDocumentType: (name: string) => invoke<DocumentType>("add_document_type", { name }),
  listDocumentTypes: () => invoke<DocumentType[]>("list_document_types"),
  deleteDocumentType: (id: string) => invoke<void>("delete_document_type", { id }),
  suggestedDocumentTypes: () => invoke<string[]>("suggested_document_types"),

  // دورة حياة الوثيقة
  updateDocumentStatus: (id: string, status: string) =>
    invoke<void>("update_document_status", { id, status }),

  // المزامنة
  syncStatus: () => invoke<SyncStatus>("sync_status"),
  hasUsers: () => invoke<boolean>("has_users"),
  setSyncKey: (passphrase: string) => invoke<void>("set_sync_key", { passphrase }),
  syncNow: () => invoke<void>("sync_now"),
};

/** يحوّل ملف (من input[type=file]) إلى base64 نظيف بدون البادئة data:...;base64, */
export function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result as string;
      resolve(result.split(",")[1]);
    };
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}

/** يبدأ تنزيل ملف في المتصفح من محتوى base64 (يُستخدم لعرض/تنزيل وثيقة بعد فك تشفيرها) */
export function downloadBase64(base64: string, filename: string, mimeType: string | null) {
  const byteChars = atob(base64);
  const byteNumbers = new Array(byteChars.length);
  for (let i = 0; i < byteChars.length; i++) byteNumbers[i] = byteChars.charCodeAt(i);
  const blob = new Blob([new Uint8Array(byteNumbers)], { type: mimeType || "application/octet-stream" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

/** رسالة خطأ نظيفة من استثناء Tauri (قد يكون كائن ArchiveError أو نصًا) */
export function errorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (e && typeof e === "object") {
    const obj = e as Record<string, unknown>;
    const key = Object.keys(obj)[0];
    if (key && typeof obj[key] === "string") return obj[key] as string;
  }
  return "حدث خطأ غير متوقع";
}

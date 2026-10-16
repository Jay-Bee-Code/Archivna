// محاكاة أوامر Tauri ببيانات عربية واقعية — لعرض الواجهة فعليًا دون الـ backend.
// تُحقن قبل تحميل الصفحة عبر add_init_script (راجع shoot.py).
(() => {
  const now = Math.floor(Date.now() / 1000), day = 86400;
  const doc = (o) => ({
    id: o.id, title: o.title, category_id: o.cat ?? null, file_hash: "a".repeat(64),
    file_size: o.size ?? 482113, mime_type: o.mime ?? "application/pdf",
    created_at: now - (o.age ?? 1) * day, updated_at: null, has_ocr: !!o.ocr,
    department_id: o.dept ?? null, document_type_id: o.type ?? null,
    registry_number: o.reg ?? null, confidentiality_level: o.conf ?? 1,
    status: o.status ?? "approved", legal_hold: !!o.hold, disposed_at: null,
    physical_location: o.loc ?? null, physical_status: o.phys ?? "none",
    borrowed_by: o.phys === "borrowed" ? "u2" : null, borrowed_at: o.phys === "borrowed" ? now - 2 * day : null,
    correspondent_id: o.corr ?? null,
  });
  const DOCX = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
  const docs = [
    doc({ id: "d1", title: "قرار تعيين رئيس مصلحة الموارد البشرية.pdf", cat: "c1", dept: "dp2", type: "t3", reg: "2026/DRH/0012-A3F2C1", conf: 2, age: 1, loc: "مبنى أ / طابق 2 / خزانة 4 / رف 3", phys: "present", corr: "k1" }),
    doc({ id: "d2", title: "مراسلة واردة من ولاية مستغانم حول الميزانية.pdf", cat: "c2", dept: "dp3", type: "t1", reg: "2026/DAF/0044-A3F2C1", age: 2, corr: "k2", status: "in_review" }),
    doc({ id: "d3", title: "عقد صفقة اقتناء التجهيزات المعلوماتية.pdf", cat: "c2", dept: "dp3", type: "t4", reg: "2026/DAF/0045-A3F2C1", conf: 3, age: 4, hold: true, phys: "borrowed", loc: "مبنى ب / خزانة 1", size: 2480000 }),
    doc({ id: "d4", title: "محضر اجتماع اللجنة التقنية 12-09-2026.docx", mime: DOCX, cat: "c3", dept: "dp1", type: "t5", reg: "2026/SG/0003-A3F2C1", age: 6, size: 91220 }),
    doc({ id: "d5", title: "مسح ضوئي - رخصة بناء رقم 118.jpg", mime: "image/jpeg", cat: "c1", ocr: true, age: 9, size: 1304552, conf: 4, status: "archived" }),
    doc({ id: "d6", title: "تقرير النشاط السنوي 2025.pdf", cat: "c3", dept: "dp1", type: "t6", reg: "2026/SG/0002-A3F2C1", age: 14, size: 5120000 }),
    doc({ id: "d7", title: "فاتورة شراء مستلزمات مكتبية.pdf", cat: "c2", dept: "dp3", type: "t7", age: 20, status: "draft", size: 210500 }),
  ];
  const cats = [
    { id: "c1", name: "الموارد البشرية", parent_id: null, created_at: now, document_count: 2 },
    { id: "c2", name: "المالية والصفقات", parent_id: null, created_at: now, document_count: 3 },
    { id: "c3", name: "الأمانة العامة", parent_id: null, created_at: now, document_count: 2 },
  ];
  const depts = [
    { id: "dp1", name_ar: "الأمانة العامة", name_fr: null, code: "SG", parent_id: null, head_user_id: null, is_active: true, created_at: now },
    { id: "dp2", name_ar: "مديرية الموارد البشرية", name_fr: null, code: "DRH", parent_id: "dp1", head_user_id: null, is_active: true, created_at: now },
    { id: "dp3", name_ar: "مديرية المالية والمحاسبة", name_fr: null, code: "DAF", parent_id: "dp1", head_user_id: null, is_active: true, created_at: now },
    { id: "dp4", name_ar: "مصلحة الصفقات العمومية", name_fr: null, code: "SMP", parent_id: "dp3", head_user_id: null, is_active: true, created_at: now },
  ];
  const types = [["t1","مراسلة واردة",5],["t2","مراسلة صادرة",5],["t3","قرار",null],["t4","عقد / صفقة",10],["t5","محضر اجتماع",null],["t6","تقرير",10],["t7","وثيقة مالية",10]]
    .map(([id, name, r]) => ({ id, name, created_at: now, retention_years: r }));
  const corrs = [
    { id: "k1", name: "وزارة الداخلية والجماعات المحلية", kind: "ministry", address: null, is_active: true, created_at: now },
    { id: "k2", name: "ولاية مستغانم", kind: "wilaya", address: null, is_active: true, created_at: now },
    { id: "k3", name: "شركة الاتصالات الجزائرية", kind: "company", address: null, is_active: true, created_at: now },
  ];
  const users = [
    { id: "u1", username: "djalil", full_name: "عبد الجليل بن سي أحمد", role: "admin", clearance_level: 4, department_id: null },
    { id: "u2", username: "samira", full_name: "سميرة بوعلام", role: "archivist", clearance_level: 3, department_id: "dp2" },
    { id: "u3", username: "karim", full_name: "كريم مزياني", role: "reviewer", clearance_level: 2, department_id: "dp3" },
    { id: "u4", username: "nadia", full_name: null, role: "viewer", clearance_level: 1, department_id: "dp1" },
  ];
  const inbox = [
    { id: "r1", document_id: "d1", document_title: docs[0].title, routed_by: "u2", routed_by_username: "samira", routed_at: now - 3600, note: "للاطلاع والمصادقة قبل نهاية الأسبوع" },
    { id: "r2", document_id: "d2", document_title: docs[1].title, routed_by: "u3", routed_by_username: "karim", routed_at: now - 2 * day, note: null },
  ];
  const sync = { configured: true, running: true, node_id: "n1", node_name: "DESKTOP-ARCHIVE-01",
    peers: [
      { node_id: "n2", name: "DESKTOP-RH-02", addr: "192.168.1.22:41234", last_seen: now, last_sync: now - 40, last_error: null },
      { node_id: "n3", name: "LAPTOP-DAF-03", addr: "192.168.1.31:40100", last_seen: now, last_sync: now - 300, last_error: null },
    ], last_sync: now - 40 };
  const qr = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 29 29" width="200" height="200"><rect width="29" height="29" fill="#fff"/><path d="M2 2h7v7H2zM20 2h7v7h-7zM2 20h7v7H2zM11 3h2v2h-2zM14 6h3v2h-3zM11 11h4v2h-4zM17 11h2v6h-2zM12 15h3v3h-3zM21 14h5v2h-5zM20 19h3v3h-3zM24 22h3v3h-3z" fill="#1A3A5C"/></svg>';
  const h = {
    current_user: () => users[0], vault_exists: () => true, ocr_status: () => window.__OCR__ ?? false,
    list_documents: () => docs, search_documents: () => docs.slice(0, 2),
    list_documents_by_category: (a) => docs.filter((d) => d.category_id === a.categoryId),
    list_categories: () => cats, list_departments: () => depts, list_document_types: () => types,
    list_correspondents: () => corrs, list_users: () => users, list_my_inbox: () => inbox,
    list_disposal_candidates: () => [docs[6], docs[5]], list_disposal_log: () => [
      { id: "l1", document_id: "x", title: "وصل استلام قديم 2014", registry_number: "2014/SG/0310-A3F2C1", disposed_by: "u1", disposed_at: now - 30 * day, reason: "انتهاء مدة الاحتفاظ" }],
    sync_status: () => sync, list_document_routing: () => [
      { id: "r1", document_id: "d1", routed_to_user_id: "u3", routed_to_username: "karim", routed_by: "u2", routed_at: now - day, note: "للمراجعة" }],
    suggested_document_types: () => ["مراسلة واردة", "قرار", "عقد / صفقة"],
    get_document_qr: () => ({ svg: qr, payload: "2026/DRH/0012-A3F2C1" }),
  };
  window.__TAURI_INTERNALS__ = {
    transformCallback: () => 0, unregisterCallback: () => {},
    invoke: async (cmd, args) => { if (!(cmd in h)) { console.warn("mock: غير معرَّف", cmd); return null; } return h[cmd](args || {}); },
  };
})();

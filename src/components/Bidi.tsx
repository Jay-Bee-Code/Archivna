import { Fragment } from "react";

/**
 * عزل المقاطع اللاتينية/الرقمية داخل النص العربي (Unicode BiDi).
 *
 * المشكلة التي يحلّها (ظهرت فعليًا بالمعاينة البصرية): في فقرة RTL، تُعيد خوارزمية
 * الاتجاهات ترتيب الأرقام وعلامات الربط، فيتحوّل رقم القيد `2026/DRH/0012-A3F2C1` إلى
 * `DRH/0012-A3F2C1/2026`، والتاريخ `12-09-2026.docx` إلى `docx.2026-09-12` — أي أن
 * البيانات تُقرأ خطأ. هنا كل مقطع من [حروف لاتينية/أرقام] تصله [. _ - / : @ +] يُعزَل
 * في <bdi dir="ltr">، فيبقى بترتيبه الصحيح ومكانه المنطقي داخل الجملة. وهو whitespace-nowrap
 * لأن المقطع إن انكسر عند الشرطة (12-09- / 2026) عاد التاريخ مقروءًا خطأ (وقع فعلًا).
 */
const LTR_RUN = /[A-Za-z0-9@][A-Za-z0-9._\-/:@+]*[A-Za-z0-9]|[A-Za-z0-9]/g;

export default function Bidi({ children }: { children: string | null | undefined }) {
  const text = children ?? "";
  const out: React.ReactNode[] = [];
  let last = 0;
  for (const m of text.matchAll(LTR_RUN)) {
    const i = m.index ?? 0;
    if (i > last) out.push(text.slice(last, i));
    out.push(
      <bdi key={i} dir="ltr" className="whitespace-nowrap">
        {m[0]}
      </bdi>
    );
    last = i + m[0].length;
  }
  if (last < text.length) out.push(text.slice(last));
  return (
    <>
      {out.map((n, i) => (
        <Fragment key={i}>{n}</Fragment>
      ))}
    </>
  );
}

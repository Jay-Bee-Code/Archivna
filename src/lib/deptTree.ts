import { Department } from "./api";

export interface FlatDeptEntry {
  dept: Department;
  depth: number;
}

/** يُسطِّح شجرة الأقسام (بترتيب DFS: كل قسم يتبعه أبناؤه مباشرة) مع عمق كل عقدة،
 * لعرضها بمسافات بادئة في قائمة منسدلة بسيطة بدل مكوّن شجري كامل. */
export function flattenDeptTree(departments: Department[]): FlatDeptEntry[] {
  const children = new Map<string | null, Department[]>();
  for (const d of departments) {
    const key = d.parent_id && departments.some((p) => p.id === d.parent_id) ? d.parent_id : null;
    if (!children.has(key)) children.set(key, []);
    children.get(key)!.push(d);
  }

  const out: FlatDeptEntry[] = [];
  function walk(parentId: string | null, depth: number) {
    for (const d of children.get(parentId) ?? []) {
      out.push({ dept: d, depth });
      walk(d.id, depth + 1);
    }
  }
  walk(null, 0);
  return out;
}

/** بادئة بصرية تعكس العمق في نص خيار <select> — "— " لكل مستوى */
export function deptIndent(depth: number): string {
  return depth > 0 ? "\u2014 ".repeat(depth) : "";
}

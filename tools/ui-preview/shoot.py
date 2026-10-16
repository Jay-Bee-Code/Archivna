"""يلتقط لقطات كل شاشات الواجهة مع بيانات محاكاة. الاستعمال:
   ./run.sh [العرض] [الارتفاع] [dark|light] [وسم]   → تُحفظ في tools/ui-preview/out/
"""
import sys, os
from playwright.sync_api import sync_playwright

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "out"); os.makedirs(OUT, exist_ok=True)
W = int(sys.argv[1]) if len(sys.argv) > 1 else 1440
H = int(sys.argv[2]) if len(sys.argv) > 2 else 900
scheme = sys.argv[3] if len(sys.argv) > 3 else "dark"
tag = sys.argv[4] if len(sys.argv) > 4 else f"{W}-{scheme}"
mock = open(os.path.join(HERE, "mock.js")).read()

with sync_playwright() as p:
    b = p.chromium.launch()
    ctx = b.new_context(viewport={"width": W, "height": H}, color_scheme=scheme, locale="ar-DZ")
    ctx.add_init_script(mock)
    pg = ctx.new_page()
    errs = []
    pg.on("console", lambda m: errs.append(m.text) if m.type in ("error", "warning") else None)
    pg.on("pageerror", lambda e: errs.append(f"pageerror: {e}"))
    pg.goto("http://localhost:4173/")
    pg.wait_for_selector("aside", timeout=8000)
    pg.wait_for_timeout(700)

    def shot(name): pg.screenshot(path=os.path.join(OUT, f"{name}-{tag}.png"))
    def nav(label):
        pg.locator("aside nav").get_by_role("button", name=label).first.click(); pg.wait_for_timeout(500)

    shot("1-dashboard")
    nav("الأرشيف"); shot("2-archive")
    pg.locator("[data-doc-open]").first.click(); pg.wait_for_timeout(400); shot("3-drawer")
    pg.mouse.click(W - 60, H // 2); pg.wait_for_timeout(300)   # الخلفية المعتمة (يمين في RTL) تغلق اللوحة
    nav("إحالاتي"); shot("4-inbox")
    nav("الهيكل التنظيمي"); shot("5-org")
    nav("الاحتفاظ والإتلاف"); shot("6-retention")
    nav("المستخدمون"); shot("7-users")
    print("أخطاء/تحذيرات المتصفح:", errs[:8] or "لا شيء")
    b.close()

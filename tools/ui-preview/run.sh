#!/bin/bash
# الخادم + اللقطات في أمر واحد. يتطلب: npm run build، ثم pip install playwright && playwright install chromium
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT/dist" && python3 -m http.server 4173 >/dev/null 2>&1 &
SRV=$!
sleep 1
cd "$ROOT/tools/ui-preview" && python3 shoot.py "$@"
kill $SRV 2>/dev/null

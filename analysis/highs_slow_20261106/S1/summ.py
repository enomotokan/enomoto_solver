"""res.jsonl を問題・タグごとに集める (時間の並び・中央値)。使い方: python summ.py [タグの頭]"""
import json, statistics, sys
from collections import defaultdict
from pathlib import Path
pre = sys.argv[1] if len(sys.argv) > 1 else ""
d = defaultdict(list)
for l in (Path(__file__).parent / "res.jsonl").read_text().splitlines():
    r = json.loads(l)
    if not r["tag"].startswith(pre) and r["tag"] != "highs":
        continue
    st = r["result"].get("status", "")
    t = r["result"].get("time") or 0
    d[(r["problem"], r["tag"])].append(f"{t:.1f}" if st.lower() == "optimal" else f"{st}")
for (p, t), v in sorted(d.items()):
    nums = [float(x) for x in v if x.replace('.', '').isdigit()]
    print(f"| {p} | {t} | {' / '.join(v)} | {statistics.median(nums):.1f} |" if nums else f"| {p} | {t} | {' / '.join(v)} | - |")

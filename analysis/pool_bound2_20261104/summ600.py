"""runs.jsonl の 600 秒の回 (l600_*) を表にする (pool_bound2、2026-11-04)。使い方: python summ600.py"""
import json
from pathlib import Path
D = Path(__file__).resolve().parent
R = {}
for l in open(D / "runs.jsonl"):
    r = json.loads(l)
    if r["name"].startswith("l600_"):
        R.setdefault(r["problem"], {})[r["name"][5:]] = r
CF = ["def", "h", "gik4"]
print("| 問題 | " + " | ".join(CF) + " |")
print("|---|" + "---|" * len(CF))
for p, d in R.items():
    cells = []
    for c in CF:
        if c not in d:
            cells.append("-"); continue
        x = d[c]["result"]
        st = x.get("status"); t = x.get("time") or 0
        cells.append(f"{st} {t:.0f}s, {x.get('objective')} / {x.get('bound') if x.get('bound') is None else round(x['bound'], 4)}, {x.get('nodes')} ノード, {d[c]['max_rss_mb']} MB")
    print(f"| {p} | " + " | ".join(cells) + " |")

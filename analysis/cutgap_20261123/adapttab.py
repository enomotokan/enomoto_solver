"""8・9 節の表: python adapttab.py → adapttab.md (adapt.jsonl の 5 問)、python adapttab.py 9 → adapttab9.md (adapt9.jsonl の 9 問)。
既定・試作・適応形 × 1・4 スレッド"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
R = {}
V = sys.argv[1] if len(sys.argv) > 1 else ""
TAGS = tuple(g + V for g in ("def", "enum", "adapt"))
PROBS = ["net12", "istanbul-no-cutoff", "ns1830653", "noswot", "mzzv42z"] + (["neos8", "nu25-pr12", "air03", "neos-960392"] if V else [])
for l in open(D / f"adapt{V}.jsonl"):
    r = json.loads(l); R[(r["problem"], r["threads"], r["tag"])] = r
def cell(r):
    if not r: return "—"
    st, obj, bd, nodes, root, secs = r["result"]
    rb = f"{float(root):.6g}" if root != "None" else "—"
    s = f"{float(secs):.1f} 秒" if st == "optimal" else f"時間切れ (暫定解 {float(obj):.6g}・下界 {float(bd):.6g})"
    return f"{s}・根 {rb}・前処理 {r.get('pre_secs', 0):.1f} 秒"
out = ["| 問題 | スレッド | 既定 | 試作 (all・loop・PRR_TIGHT) | 適応形 (ENUM_ADAPT=1) |", "|---|---:|---|---|---|"]
for p in PROBS:
    for t in (1, 4):
        out.append(f"| {p} | {t} | " + " | ".join(cell(R.get((p, t, g))) for g in TAGS) + " |")
(D / f"adapttab{V}.md").write_text("\n".join(out) + "\n")
print("\n".join(out))

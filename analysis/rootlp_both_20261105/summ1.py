"""batch1.txt (neos859080・beasleyC3・n2seq36q を def・rob・both で) の集計。根の LP の勝ち負けと試作の判断はログ (runs/*.log) から。"""
import json, re, statistics, sys
from pathlib import Path
D = Path(__file__).resolve().parent
rows = {}
for l in open(D / "batch1.txt"):
    if " " not in l.strip():
        continue
    n, j = l.split(" ", 1)
    try:
        r = json.loads(j)
    except Exception:
        continue
    p, c, s = n.rsplit("_", 2)
    rows.setdefault((p, c), []).append((s, r))
for (p, c), v in rows.items():
    ts = [r["time"] for _, r in v]
    print(f"{p:11s} {c:5s} n={len(v):2d} 中央値 {statistics.median(ts):7.1f} 最悪 {max(ts):7.1f} 最良 {min(ts):6.1f} 時間切れ {sum(r['status']=='time_limit' for _, r in v)}")
print()
for (p, c), v in rows.items():
    for s, r in v:
        log = (D / "runs" / f"{p}_{c}_{s}.log")
        txt = log.read_text(errors="replace") if log.exists() else ""
        lost = re.search(r"root LP both: the (\w+) side lost", txt)
        dec = re.search(r"root LP both: probe gain .* -> (.*)", txt)
        nov = "no second vertex" in txt
        win = "ipm" if "interior-point method + crossover finished first" in txt.split("solving the root LP")[-1] else "simplex"
        extra = ""
        if c == "both":
            extra = f" 負け {lost.group(1) if lost else '-'} 判断 {dec.group(1) if dec else ('頂点なし' if nov else '-')}"
        print(f"{p:11s} {c:5s} {s:4s} {r['status']:10s} {r['time']:7.1f} 根 {r['root_bound']:.4f} 節 {r['nodes']}{extra}")

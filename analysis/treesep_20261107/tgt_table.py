"""ターゲットの 600 秒の比べ: python tgt_table.py tgt600 [設定...]"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
pre = sys.argv[1]
cfgs = sys.argv[2:] or ["def"]
R = {}
for l in open(D / f"{pre}.jsonl"):
    r = json.loads(l); R[(r["problem"], r["cfg"], r["rep"])] = r["result"]
probs = list(dict.fromkeys(k[0] for k in R))
pat = re.compile(r"ts rounds (\d+) added (\d+) rows (\d+)")
def gap(x):
    o, b = x.get("objective"), x.get("bound")
    if o is None or b is None:
        return "-"
    return f"{abs(o-b)/max(abs(o),1):.2e}"
print("| 問題 | 種 | " + " | ".join(f"{c}: 状態 秒 ギャップ ノード" for c in cfgs) + " |")
print("|---|---|" + "---|" * len(cfgs))
for p in probs:
    for s in sorted({k[2] for k in R if k[0] == p}):
        cells = []
        for c in cfgs:
            x = R.get((p, c, s))
            cells.append("-" if x is None else f"{x['status'][:4]} {x['time']:.0f} {gap(x)} {x.get('nodes')}")
        print(f"| {p} | {s} | " + " | ".join(cells) + " |")

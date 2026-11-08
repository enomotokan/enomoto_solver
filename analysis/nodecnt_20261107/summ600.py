"""e600.jsonl (600 秒、設定ごと・種ごと) の表: 状態・秒・下界・暫定解・ギャップ・ノード。使い方: python summ600.py [jsonl]"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
fn = Path(sys.argv[1]) if len(sys.argv) > 1 else D / "e600.jsonl"
R = {}
for l in open(fn):
    r = json.loads(l); R[(r["problem"], r["rep"], r["cfg"])] = r["result"]
probs = sorted({k[0] for k in R}, key=lambda p: [k[0] for k in R].index(p))
cfgs = sorted({k[2] for k in R})
print("| 問題 | 種 | " + " | ".join(f"{c}: 状態 秒 下界 解 ギャップ ノード" for c in cfgs) + " |")
print("|---|---|" + "---|" * len(cfgs))
for p in probs:
    for s in sorted({k[1] for k in R if k[0] == p}):
        cells = []
        for c in cfgs:
            x = R.get((p, s, c))
            if not x:
                cells.append("-"); continue
            ub, lb = x.get("objective"), x.get("bound")
            gap = abs(ub - lb) / max(1e-9, abs(ub)) if (ub is not None and lb is not None) else float("nan")
            st = "**最適**" if x.get("status") == "optimal" else x.get("status")
            cells.append(f"{st} {x.get('time', 0):.0f} {lb:.10g} {ub} {gap:.2e} {x.get('nodes')}")
        print(f"| {p} | {s} | " + " | ".join(cells) + " |")

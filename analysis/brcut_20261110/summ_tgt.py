"""(a) 効きそうな問題 (tgt.jsonl) の表: 問題・種ごとに設定ごとの (時間, ノード, 状態, 下界)。使い方: python summ_tgt.py [FILE]"""
import json, sys
from collections import defaultdict
from pathlib import Path
f = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent / "tgt.jsonl"
R = defaultdict(dict); cfgs = []
for l in open(f):
    if not l.strip(): continue
    r = json.loads(l); R[(r["problem"], r["rep"])][r["cfg"]] = r["result"]
    if r["cfg"] not in cfgs: cfgs.append(r["cfg"])
print("| 問題 | 種 | " + " | ".join(f"{c} 時間 / ノード / 下界" for c in cfgs) + " |")
print("|---|---|" + "---|" * len(cfgs))
for k in sorted(R):
    cells = []
    for c in cfgs:
        x = R[k].get(c)
        if x is None: cells.append("-"); continue
        st = "" if x.get("status") == "optimal" else " (打切)"
        cells.append(f"{x.get('time', 0):.1f}{st} / {x.get('nodes')} / {x.get('bound'):.6g}")
    print(f"| {k[0]} | {k[1]} | " + " | ".join(cells) + " |")

"""9.6 節の表: cmp_t.jsonl (と mc11 の既定・r は mc11.jsonl) から、問題・スレッドごとに既定・r・t の秒 (種 1 / 2 / 3、括弧は幾何平均、
時間切れは時間の上限で数える) とノードを出す。使い方 (リポジトリの根から): python3 analysis/nodespeed_20261123/cmp_t_sum.py"""
import collections, json, math
from pathlib import Path
D = Path(__file__).resolve().parent
R = collections.defaultdict(dict)
for f, src in [("mc11.jsonl", "m"), ("cmp_t.jsonl", "c")]:
    for l in open(D / f):
        d = json.loads(l)
        p, c = ("mc11", {"def": "def", "adr": "r"}[d["tag"]]) if src == "m" else (d["problem"], d["cfg"])
        r = d["result"]
        R[(p, d["threads"])][(c, d["seed"])] = (r["status"] in ("optimal", "infeasible"), r["time"], r["nodes"])
print("| 問題 | スレッド | 既定 | r | t |")
print("|---|---|---|---|---|")
for k in sorted(R):
    tl = 200 if k[0].startswith("fhnw") else 120
    cells = []
    for c in ["def", "r", "t"]:
        v = [R[k].get((c, s)) for s in (1, 2, 3)]
        txt = " / ".join((f"{x[1]:.1f}" if x[0] else "TL") if x else "-" for x in v)
        ts = [x[1] if x[0] else tl for x in v if x]
        g = math.exp(sum(map(math.log, ts)) / len(ts)) if ts else 0
        nn = " / ".join(str(x[2]) for x in v if x)
        cells.append(f"{txt} (**{g:.1f}**)<br>{nn}")
    print(f"| {k[0]} | {k[1]} | " + " | ".join(cells) + " |")

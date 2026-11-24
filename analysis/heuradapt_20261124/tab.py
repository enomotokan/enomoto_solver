"""res_one.jsonl の 1 スレッドの行を問題 × 種 × 札の表にする: python tab.py [札 ...]。解けた回は秒、時間切れは TL(暫定解)。
最後に札ごとの、全部の札がそろった (問題, 種) でのずらし幾何平均 (ずらし 10 秒、時間切れは上限の秒) を出す。"""
import json, math, sys
from pathlib import Path
tags = sys.argv[1:] or ["def", "fl", "fps", "ad"]
R = {}
for l in open(Path(__file__).parent / "res_one.jsonl"):
    r = json.loads(l)
    if r["threads"] == 1 and r["result"]:
        R[(r["problem"], r["tag"], r["seed"])] = r["result"]
probs = sorted({p for p, t, s in R if t in tags})
print("| 問題 | 種 | " + " | ".join(tags) + " |")
print("|---|---:|" + "---|" * len(tags))
full = []
for p in probs:
    for s in (1, 2, 3):
        xs = [R.get((p, t, s)) for t in tags]
        if not any(xs):
            continue
        cell = lambda x: "-" if x is None else (f"{x['time']:.1f}" if x["status"] in ("optimal", "infeasible") else f"TL({x['objective']:.6g})" if x.get("objective") is not None else "TL(なし)")
        print(f"| {p} | {s} | " + " | ".join(cell(x) for x in xs) + " |")
        if all(xs):
            full.append(xs)
def T(x):
    return x["time"] if x["status"] in ("optimal", "infeasible") else x["time"]
print()
for i, t in enumerate(tags):
    g = math.exp(sum(math.log(T(xs[i]) + 10) for xs in full) / len(full)) - 10
    sol = sum(xs[i]["status"] in ("optimal", "infeasible") for xs in full)
    print(f"{t}: そろった {len(full)} 回、解けた {sol}、ずらし幾何平均 {g:.2f} 秒")

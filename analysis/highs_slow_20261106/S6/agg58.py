"""bench58.jsonl の集計: 設定 x rep ごとの解けた数 (optimal かつ 60 秒以内)・SGM (ずらし 10 秒、時間切れは 60 秒)・crash、
設定のあいだで解けたかが違う問題・時間が 1.5 倍以上違う問題 (どちらかが 5 秒以上)。使い方: python agg58.py 基準 比べる..."""
import json, math, sys, collections
from pathlib import Path
D = Path(__file__).resolve().parent
base, others = sys.argv[1], sys.argv[2:]
R = collections.defaultdict(dict)
for l in open(D / "bench58.jsonl"):
    r = json.loads(l); R[(r["cfg"], r["rep"])][r["problem"]] = r["result"]
def t(x): return x["time"] if x.get("status") == "optimal" and x.get("time", 1e9) <= 60.5 else 60.0
reps = sorted({k[1] for k in R})
for rep in reps:
    common = set.intersection(*[set(R[(c, rep)]) for c in [base] + others if (c, rep) in R])
    print(f"rep {rep}: 共通 {len(common)} 問")
    for c in [base] + others:
        if (c, rep) not in R: continue
        v = [t(R[(c, rep)][p]) for p in common]
        sgm = math.exp(sum(math.log(x + 10) for x in v) / len(v)) - 10
        crash = sum(1 for p in common if R[(c, rep)][p].get("status") == "crash")
        print(f"  {c}: 解けた {sum(1 for x in v if x < 60)}  SGM {sgm:.2f}  crash {crash}")
    for c in others:
        if (c, rep) not in R: continue
        for p in sorted(common):
            a, b = t(R[(base, rep)][p]), t(R[(c, rep)][p])
            if (a < 60) != (b < 60) or (max(a, b) >= 5 and max(a, b) / max(min(a, b), 1e-9) >= 1.5):
                print(f"    {p}: {base} {a:.1f} / {c} {b:.1f}")

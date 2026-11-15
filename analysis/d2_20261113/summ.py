"""D2 の集計: python summ.py [P|M]  (res_SET_s*.jsonl を読む)
設定ごとに解けた数・時間の shifted geomean (shift 10、解けなければ制限時間) と base との比 (同じ (問題, 種) の対だけ)、
最適と言った目的値が問題ごとに食い違う回 (誤答の候補)。M は問題ごとの表 (時間・目的値・下界・ノード)。"""
import glob, json, math, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
SET = sys.argv[1] if len(sys.argv) > 1 else "P"
R = {}
for f in sorted(glob.glob(str(D / f"res_{SET}_s*.jsonl"))):
    for l in open(f):
        if l.strip():
            r = json.loads(l); R[(r["problem"], r["cfg"], r["seed"])] = r
cfgs = sorted({k[1] for k in R}, key=lambda c: (c != "base", c))
def t(r):
    res = r["result"]
    return min(res.get("time") or r.get("tl", 60), r.get("tl", 60)) if res.get("status") == "optimal" else r.get("tl", 60)
print(f"{SET}: {len(R)} 回")
for c in cfgs:
    keys = [k for k in R if k[1] == c]
    solved = sum(R[k]["result"].get("status") == "optimal" for k in keys)
    pair = [(k[0], k[2]) for k in keys if (k[0], "base", k[2]) in R]
    lr = [math.log((t(R[(p, c, s)]) + 10) / (t(R[(p, "base", s)]) + 10)) for p, s in pair]
    sg = math.exp(sum(math.log(t(R[k]) + 10) for k in keys) / max(len(keys), 1)) - 10
    print(f"  {c:6s} 回 {len(keys):4d} 解けた {solved:4d} sgm {sg:7.2f}  base との比 (対 {len(pair)}) {math.exp(sum(lr) / max(len(lr), 1)) - 1:+.1%}")
opt = defaultdict(list)
for k, r in R.items():
    if r["result"].get("status") == "optimal" and r["result"].get("objective") is not None:
        opt[k[0]].append((r["result"]["objective"], k[1], k[2]))
bad = [(p, v) for p, v in opt.items() if max(x[0] for x in v) - min(x[0] for x in v) > 1e-6 * max(1.0, max(abs(x[0]) for x in v))]
print("最適の目的値の食い違い:", bad if bad else "なし")
if SET == "M":
    for p in sorted({k[0] for k in R}):
        print(p, " | ".join(f"{c}: {R[(p, c, 1)]['result'].get('status')} {R[(p, c, 1)]['result'].get('time', 0):.0f}s obj {R[(p, c, 1)]['result'].get('objective')} lb {R[(p, c, 1)]['result'].get('bound')} n {R[(p, c, 1)]['result'].get('nodes')}" for c in cfgs if (p, c, 1) in R))

"""D2 適応版の集計: python summ.py SET [d2 の SET]  (res_SET_s*.jsonl を読む。2 つ目を与えると analysis/d2_20261113/res_<それ>_s*.jsonl の
base・d2b を、この SET に無い (問題, 設定, 種) に限って `base0`・`d2b0` (前の回、機械も違いうる) として足す)
設定ごとに解けた数・shifted geomean (shift 10、解けなければ制限時間) と base との対の比 (95% 区間は対の log 比の t 近似)、
問題ごとの比 (種の平均)、種をまたいで一貫して遅い問題 (全種で比 > 1.1)、最適と言った目的値の食い違い、D2b を使い始めたスレッドの数。"""
import glob, json, math, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
SET = sys.argv[1]
R = {}
for f in sorted(glob.glob(str(D / f"res_{SET}_s*.jsonl"))):
    for l in open(f):
        if l.strip():
            r = json.loads(l); R[(r["problem"], r["cfg"], r["seed"])] = r
if len(sys.argv) > 2:
    for f in sorted(glob.glob(str(D.parent / "d2_20261113" / f"res_{sys.argv[2]}_s*.jsonl"))):
        for l in open(f):
            r = json.loads(l)
            if r["cfg"] in ("base", "d2b"):
                R[(r["problem"], r["cfg"] + "0", r["seed"])] = r
cfgs = sorted({k[1] for k in R}, key=lambda c: (c != "base", c))
def t(r):
    res = r["result"]
    return min(res.get("time") or r.get("tl", 60), r.get("tl", 60)) if res.get("status") == "optimal" else r.get("tl", 60)
def on(r):
    m = [re.search(r"threads switched on (\d+)", l) for l in r["result"].get("d2log", [])]
    m = [int(x[1]) for x in m if x]
    return m[-1] if m else None
print(f"{SET}: {len(R)} 回")
for c in cfgs:
    keys = [k for k in R if k[1] == c]
    solved = sum(R[k]["result"].get("status") == "optimal" for k in keys)
    crash = sum(R[k]["result"].get("status") not in ("optimal", "time_limit", "infeasible") for k in keys)
    pair = [(k[0], k[2]) for k in keys if (k[0], "base", k[2]) in R]
    lr = [math.log((t(R[(p, c, s)]) + 10) / (t(R[(p, "base", s)]) + 10)) for p, s in pair]
    sg = math.exp(sum(math.log(t(R[k]) + 10) for k in keys) / max(len(keys), 1)) - 10
    n = len(lr); mu = sum(lr) / max(n, 1)
    sd = math.sqrt(sum((x - mu) ** 2 for x in lr) / (n - 1)) if n > 1 else 0.0
    ci = f"[{math.exp(mu - 1.96 * sd / math.sqrt(n)) - 1:+.1%}, {math.exp(mu + 1.96 * sd / math.sqrt(n)) - 1:+.1%}]" if n > 1 else ""
    ons = [on(R[k]) for k in keys if on(R[k])]
    print(f"  {c:6s} 回 {len(keys):4d} 解けた {solved:4d} crash {crash} sgm {sg:7.2f}  base との比 (対 {n}) {math.exp(mu) - 1:+.1%} {ci}  D2b を使い始めた回 {len(ons)}")
    if c == "base" or not pair:
        continue
    per = defaultdict(list)
    for p, s in pair:
        per[p].append(math.log((t(R[(p, c, s)]) + 10) / (t(R[(p, "base", s)]) + 10)))
    sv = defaultdict(lambda: [0, 0])
    for p, s in pair:
        sv[p][0] += R[(p, "base", s)]["result"].get("status") == "optimal"; sv[p][1] += R[(p, c, s)]["result"].get("status") == "optimal"
    worse = sorted((p for p in per if all(x > math.log(1.1) for x in per[p]) and len(per[p]) > 1), key=lambda p: -sum(per[p]))
    big = sorted(per, key=lambda p: -abs(sum(per[p]) / len(per[p])))[:8]
    print("    問題ごとの比 (大きい順):", ", ".join(f"{p} {math.exp(sum(per[p]) / len(per[p])) - 1:+.0%}" for p in big))
    print("    全種で 10% 超遅い:", worse or "なし", " 解けた数の差:", {p: f"{v[0]}→{v[1]}" for p, v in sv.items() if v[0] != v[1]})
    swp = sorted({k[0] for k in keys if on(R[k])})
    if swp:
        print("    D2b を使い始めた問題:", ", ".join(f"{p} ({','.join(str(k[2]) for k in keys if k[0] == p and on(R[k]))})" for p in swp))
opt = defaultdict(list)
for k, r in R.items():
    if r["result"].get("status") == "optimal" and r["result"].get("objective") is not None:
        opt[k[0]].append((r["result"]["objective"], k[1], k[2]))
bad = [(p, v) for p, v in opt.items() if max(x[0] for x in v) - min(x[0] for x in v) > 1e-6 * max(1.0, max(abs(x[0]) for x in v))]
print("最適の目的値の食い違い:", bad if bad else "なし")
if SET.startswith("M") or SET.startswith("P1"):
    for p in sorted({k[0] for k in R}):
        for s in sorted({k[2] for k in R if k[0] == p}):
            print(f"{p} 種{s}", " | ".join(f"{c}: {R[(p, c, s)]['result'].get('status')[:4]} {R[(p, c, s)]['result'].get('time') or 0:.0f}s lb {R[(p, c, s)]['result'].get('bound')} ub {R[(p, c, s)]['result'].get('objective')} n {R[(p, c, s)]['result'].get('nodes')} on {on(R[(p, c, s)])}" for c in cfgs if (p, c, s) in R))

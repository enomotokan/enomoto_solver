"""θ の掃引の集計: python summ.py > summ.txt。設定ごとに解けた数・def とのずらし幾何平均の比 (1 秒ずらし、どれかが解けた (問題, 種)、95% 区間は問題単位のブートストラップ)、
問題ごとに設定別の時間 (2 種の幾何平均、時間切れは * ) と、主スレッドの外した行・戻した行の数 (2 種の平均)、外した非零の割合を出す。"""
import json, glob, math, random, re, collections, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
from cfg import CFGS  # noqa: E402
R = {}
for f in sorted(glob.glob(str(D / "res_s*.jsonl"))):
    for l in open(f):
        if l.strip():
            r = json.loads(l); R[(r["problem"], r["cfg"], r["seed"])] = r
def ok(r): return (r["result"].get("status") or "").lower() in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if ok(r) else r["tl"]
def stats(r):
    d = re.search(r"CLQ_LAZY: (\d+) clique rows dropped from the LP \(of (\d+) rows, (\d+) nonzeros\)", " ".join(r["lines"]))
    s = re.search(r"CLQ_LAZY stats tid 0: dropped (\d+) checks (\d+) restored (\d+) resolves (\d+)", " ".join(r["lines"]))
    return (int(d.group(1)) if d else 0, int(d.group(3)) if d else 0, int(s.group(3)) if s else 0, int(s.group(4)) if s else 0)
probs = sorted({k[0] for k in R}); seeds = sorted({k[2] for k in R}); cfgs = [c for c in CFGS if any(k[1] == c for k in R)]
bad = [(p, s) for p in probs for s in seeds if len({round(R[(p, c, s)]["result"]["objective"], 4) for c in cfgs if (p, c, s) in R and (R[(p, c, s)]["result"].get("status") or "") == "optimal"}) > 1]
print(f"回 {len(R)}、問題 {len(probs)}、種 {seeds}、crash {sum(1 for r in R.values() if (r['result'].get('status') or '').startswith('crash'))}、最適値の食い違い {bad}\n")
print("## 設定ごと (def との比)")
for c in cfgs:
    per = collections.defaultdict(list); n_ok = 0
    for p in probs:
        for s in seeds:
            a, b = R.get((p, "def", s)), R.get((p, c, s))
            if not b: continue
            n_ok += ok(b)
            if a and (ok(a) or ok(b)): per[p].append(math.log((t(b) + 1) / (t(a) + 1)))
    v = [x for xs in per.values() for x in xs]
    g = math.exp(sum(v) / len(v)) if v else float("nan")
    rnd = random.Random(0); keys = list(per); bs = []
    for _ in range(2000):
        smp = [x for _ in keys for x in per[rnd.choice(keys)]]
        if smp: bs.append(math.exp(sum(smp) / len(smp)))
    bs.sort(); lo, hi = (bs[int(0.025 * len(bs))], bs[int(0.975 * len(bs)) - 1]) if bs else (float("nan"),) * 2
    print(f"{c:4s}: 解けた {n_ok:3d}  比 {g:.3f} [{lo:.3f}, {hi:.3f}]")
print("\n## 問題ごと (秒、2 種の幾何平均、* は時間切れを含む) / 外した行・戻した行 (2 種の平均)")
print(f"{'問題':22s} " + " ".join(f"{c:>13s}" for c in cfgs))
for p in probs:
    cells = []
    for c in cfgs:
        rs = [R[(p, c, s)] for s in seeds if (p, c, s) in R]
        if not rs: cells.append(f"{'-':>13s}"); continue
        g = math.exp(sum(math.log(t(r) + 1) for r in rs) / len(rs)) - 1
        st = [stats(r) for r in rs]
        dr = sum(x[0] for x in st) / len(st); rs_ = sum(x[2] for x in st) / len(st)
        cells.append(f"{g:6.1f}{'*' if not all(ok(r) for r in rs) else ' '}{'' if c == 'def' else f'{dr:.0f}/{rs_:.0f}':>6s}")
    print(f"{p:22s} " + " ".join(f"{x:>13s}" for x in cells))

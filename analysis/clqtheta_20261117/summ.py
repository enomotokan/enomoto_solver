"""θ の掃引の集計: python summ.py > summ.txt。設定ごとに解けた数・def とのずらし幾何平均の比 (1 秒ずらし、どれかが解けた (問題, 種)、95% 区間は問題単位のブートストラップ)、
問題ごとに設定別の時間 (2 種の幾何平均、時間切れは * ) と、主スレッドの外した行・戻した行の数 (2 種の平均) を出す。
最後に、問題の特徴 (t1 = 今の lazy で外す行の非零 / 根の LP の非零、`clqrows_20261117` の根の LP の非零で割る) と θ ごとの効き目 (def との比) の表。"""
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

# 特徴と効き目: 外す非零の割合 (t1 の主の外した非零 / 根の LP の非零) と θ ごとの比
NNZ = {}
for l in open(D.parent / "clqrows_20261117/res.jsonl"):
    r = json.loads(l)
    for x in r["lines"]:
        m = re.search(r"CLQROWS: rows (\d+) nnz (\d+)", x)
        if m: NNZ[r["problem"]] = (int(m.group(1)), int(m.group(2)))
print("\n## 特徴 (t1 で外す行の数・非零の割合、戻した行 / 外した行) と θ ごとの比 (def との時間の比、2 種の幾何平均)")
print(f"{'問題':22s} {'外す行%':>7s} {'非零%':>6s} {'戻し率':>6s} | " + " ".join(f"{c:>6s}" for c in cfgs if c != "def"))
feat = []
for p in probs:
    rs = [R[(p, "t1", s)] for s in seeds if (p, "t1", s) in R]
    if not rs or p not in NNZ: continue
    st = [stats(r) for r in rs]
    dr = sum(x[0] for x in st) / len(st); dn = sum(x[1] for x in st) / len(st); rb = sum(x[2] for x in st) / len(st)
    rows_, nnz_ = NNZ[p]
    ratios = []
    for c in cfgs:
        if c == "def": continue
        v = [math.log((t(R[(p, c, s)]) + 1) / (t(R[(p, "def", s)]) + 1)) for s in seeds if (p, c, s) in R and (p, "def", s) in R]
        ratios.append(math.exp(sum(v) / len(v)) if v else float("nan"))
    feat.append((dn / nnz_, p))
    print(f"{p:22s} {100*dr/rows_:6.1f}% {100*dn/nnz_:5.1f}% {rb/max(dr,1):6.2f} | " + " ".join(f"{x:6.2f}" for x in ratios))

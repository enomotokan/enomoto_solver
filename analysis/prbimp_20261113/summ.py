"""PRBIMP の対の集計。python summ.py SET A B [N58]: res_SET_s*.jsonl の (問題, 種) ごとに A と B を比べる (B / A)。
SGM は shift 1 秒、時間切れは制限時間で数える。CI は問題単位のブートストラップ (4000 回、5%・95% ではなく 2.5%・97.5%)。
N58 を与えると、計測していない問題 (前処理が A と同じになるので比 1 とみなす) を N58 - 計測した問題の数だけ比 1 で足した値も出す。"""
import json, glob, math, random, collections, sys
from pathlib import Path
D = Path(__file__).resolve().parent
SET, A, B = sys.argv[1], sys.argv[2], sys.argv[3]
N58 = int(sys.argv[4]) if len(sys.argv) > 4 else 0
rows = [json.loads(l) for f in sorted(glob.glob(str(D / f"res_{SET}_s*.jsonl"))) for l in open(f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
def solved(r): return r["result"].get("status") in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
probs = sorted({p for p, c, s in R}); seeds = sorted({s for p, c, s in R})
logr = collections.defaultdict(list); ns = {A: 0, B: 0}; crash = 0; bad = []
for p in probs:
    for s in seeds:
        a, b = R.get((p, A, s)), R.get((p, B, s))
        if not a or not b: continue
        for x in (a, b):
            if x["result"].get("status") in (None, "crash", "error", "fetch_error"): crash += 1
        ns[A] += solved(a); ns[B] += solved(b)
        if a["result"].get("status") == "optimal" and b["result"].get("status") == "optimal":
            oa, ob = a["result"]["objective"], b["result"]["objective"]
            if abs(oa - ob) > 1e-6 * max(1, abs(oa)): bad.append((p, s, oa, ob))
        if {a["result"].get("status"), b["result"].get("status")} == {"optimal", "infeasible"}: bad.append((p, s, "infeas"))
        if solved(a) or solved(b): logr[p].append(math.log((t(b) + 1) / (t(a) + 1)))
def sgm(ps, extra=0):
    v = [x for p in ps for x in logr[p]]; n = len(v) + extra
    return math.exp(sum(v) / n) if n else 1.0
ps = list(logr); random.seed(0)
est = sgm(ps); bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000))
print(f"{SET}: 行数 {len(rows)}、crash {crash}、食い違い {len(bad)} {bad}")
print(f"解けた数: {A} {ns[A]} / {B} {ns[B]}")
print(f"{B}/{A} (どちらかが解けた {len(ps)} 問): {est-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}]")
if N58:
    k = len(seeds); ones = [f"_one{i}" for i in range(N58 - len(probs))]
    for o in ones: logr[o] = [0.0] * k
    pa = ps + ones; est2 = sgm(pa); bs2 = sorted(sgm([random.choice(pa) for _ in pa]) for _ in range(4000))
    print(f"  計測していない {len(ones)} 問を比 1 (種 {k} 個ずつ) で足すと: {est2-1:+.1%} [{bs2[100]-1:+.1%}, {bs2[3899]-1:+.1%}]")
print("問題ごとの比 (種の順):")
for p in ps:
    v = [math.exp(x) for x in logr[p]]
    mark = " 揃って遅" if len(v) >= 2 and all(x > 1.25 for x in v) else (" 揃って速" if len(v) >= 2 and all(x < 0.8 for x in v) else "")
    ta = " ".join(f"{t(R[p, A, s]):.1f}" if (p, A, s) in R else "-" for s in seeds); tb = " ".join(f"{t(R[p, B, s]):.1f}" if (p, B, s) in R else "-" for s in seeds)
    print(f"  {p:22s} {' '.join(f'{x:.2f}' for x in v)}  ({A} {ta} | {B} {tb}){mark}")
for c in (A, B):
    print(c, "最大 RSS", max((r.get("max_rss_mb", 0) for r in rows if r["cfg"] == c), default=0), "MB")

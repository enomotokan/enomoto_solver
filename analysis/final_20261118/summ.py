"""最終計測の集計: python summ.py M (または P) > summ_M.txt
今の既定 (この台本の def・def_t1) を、2026-11-17 の 0.1.9 の既定・SCIP・HiGHS の行 (同じ種) と比べる。
- 今 def / v019 (PyPI 0.1.9、この台本で同じ機械・対)、今 def_t1 / SCIP (どちらも 1 スレッド)、今 def / SCIP・HiGHS (vshighs)、
  今 def_t1 / o019_t1 (11-17 の 0.1.9 の 1 スレッド)
- 揺れの目安: v019 / o019 (同じ 0.1.9 の今日と 11-17)、o019 (vsscip) / o019h (vshighs) (同じ版・同じ種の別の日の 2 回)
SGM は shift 1 秒、時間切れは上限で数える。CI は問題単位のブートストラップ (vshighs の summ.py と同じ)。
食い違い: 最適と答えた目的値が相手と相対 1e-6 を超えて違う、または最適と実行不能が混在。"""
import json, glob, math, random, collections, sys
from pathlib import Path
D = Path(__file__).resolve().parent
SET = sys.argv[1] if len(sys.argv) > 1 else "M"
def load(d, sets, rename=None, only=None):
    R = {}
    for S in sets:
        for f in sorted(glob.glob(str(D.parent / d / f"res_{S}_s*.jsonl"))):
            for l in open(f):
                if not l.strip(): continue
                r = json.loads(l); c = r["cfg"]
                if only and c not in only: continue
                c = (rename or {}).get(c, c)
                if S in ("M", "A") and r["seed"] != 1: continue  # M は種 1 だけでそろえる
                R[(r["problem"], c, r["seed"])] = r
    return R
src = "MA" if SET == "M" else "P"
R = {}
R.update(load("final_20261118", [SET]))
R.update(load("vsscip_20261117", ["M" if SET == "M" else "P"], {"def": "o019", "def_t1": "o019_t1"}))
R.update(load("vshighs_20261117", ["M", "A"] if SET == "M" else ["P"], {"def": "o019h"}))
def st(r): return str(r["result"].get("status") or "").lower()
def solved(r): return st(r) in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
cfgs = sorted({c for _, c, _ in R}); probs = sorted({p for p, c, _ in R if c == "def"}); seeds = sorted({s for p, c, s in R if c == "def"})
mine = [r for (p, c, s), r in R.items() if c in ("def", "def_t1", "v019")]
crash = [(r["problem"], r["cfg"], r["seed"]) for r in mine if "error" in r["result"] or st(r) in ("", "crash", "error", "fetch_error")]
print(f"{SET}: 今の行 {len(mine)}、問題 {len(probs)}、種 {seeds}、設定 {cfgs}、crash {len(crash)} {crash}")
for c in ("def", "def_t1", "v019"):
    print(c, "最大 RSS", max([(r.get("max_rss_mb") or 0) for r in mine if r["cfg"] == c] or [0]), "MB")
# 食い違い (今の def・def_t1 を全相手と)
bad = []
for p in probs:
    for s in seeds:
        rs = {c: R[(p, c, s)] for c in cfgs if (p, c, s) in R}
        opt = {c: r["result"].get("objective") for c, r in rs.items() if st(r) == "optimal" and r["result"].get("objective") is not None}
        for c in ("def", "def_t1"):
            if c not in opt: continue
            for o, v in opt.items():
                if o != c and abs(v - opt[c]) > 1e-6 * max(1, abs(v)): bad.append((p, s, c, opt[c], o, v))
            if any(st(r) == "infeasible" for r in rs.values()): bad.append((p, s, c, "opt/inf"))
print("食い違い", len(bad)); [print("  ", b) for b in bad]
def cmp(ca, cb, show=25):
    full = [(p, s) for p in probs for s in seeds if (p, ca, s) in R and (p, cb, s) in R]
    logr = collections.defaultdict(list)
    for p, s in full:
        a, b = R[(p, ca, s)], R[(p, cb, s)]
        if solved(a) or solved(b): logr[p].append(math.log((t(a) + 1) / (t(b) + 1)))
    ps = list(logr)
    def sgm(q):
        v = [x for p in q for x in logr[p]]; return math.exp(sum(v) / len(v)) if v else 1.0
    random.seed(0); bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000)) if ps else [1.0] * 4000
    sa = sum(solved(R[(p, ca, s)]) for p, s in full); sb = sum(solved(R[(p, cb, s)]) for p, s in full)
    print(f"\n== {ca} / {cb} ({len(full)} 組、どちらかが解けた {len(ps)} 問): {sgm(ps)-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}]、解けた {ca} {sa} / {cb} {sb}")
    only_b = [(p, s) for p, s in full if solved(R[(p, cb, s)]) and not solved(R[(p, ca, s)])]
    only_a = [(p, s) for p, s in full if solved(R[(p, ca, s)]) and not solved(R[(p, cb, s)])]
    print(f"  {cb} だけ解けた {len(only_b)}: {only_b}")
    print(f"  {ca} だけ解けた {len(only_a)}: {only_a}")
    order = sorted(ps, key=lambda q: -sum(logr[q]) / len(logr[q]))
    print(f"  {ca} が遅い順 (比 > 1.25、上位 {show}):")
    for p in order[:show]:
        g = math.exp(sum(logr[p]) / len(logr[p]))
        if g <= 1.25: break
        print(f"    {p:32s} 比 {g:6.2f}  " + "  ".join(f"s{s}: {t(R[(p, ca, s)]):6.1f}/{t(R[(p, cb, s)]):6.1f}" for s in seeds if (p, ca, s) in R and (p, cb, s) in R))
for ca, cb in [("def", "v019"), ("def_t1", "scip"), ("def", "scip"), ("def", "highs"), ("def_t1", "o019_t1"), ("v019", "o019"), ("o019", "o019h")]:
    cmp(ca, cb)

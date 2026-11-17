"""伝播の閾値の確かめの集計 (def と rt)。python summ.py P > summ_P.txt、python summ.py M > summ_M.txt、
python summ.py PM > summ_PM.txt (採用条件の表だけ)。SGM は shift 1 秒、時間切れは制限時間で数える。CI は問題単位のブートストラップ (4000 回、95%)。
採用条件 (これまでの束と同じ): (1) 試作 / def の SGM の 95% CI の上限、(2) 解けた数 (P + M で減らない)、(3) 全種で 1.25 倍を超えて遅い問題、
(4) 食い違い (3 設定のあいだの最適値の違い・最適と実行不能の混在) 0、crash 0。"""
import json, glob, math, random, collections, sys
from pathlib import Path
D = Path(__file__).resolve().parent
ARG = sys.argv[1] if len(sys.argv) > 1 else "P"
BS = ("rtc",) if ARG in ("P2", "PX", "MX", "PXMX") else ("rt",)
CF = ["def", "rt", "rtc"]
def st(r): return str(r["result"].get("status") or "").lower()
def solved(r): return st(r) in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
def load(S):
    rows = [json.loads(l) for f in sorted(glob.glob(str(D / f"res_{S}_s*.jsonl"))) for l in open(f) if l.strip()]
    return rows, {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
def summ(S, B, verbose=True):
    rows, R = load(S)
    probs = sorted({p for p, c, s in R}); seeds = sorted({s for p, c, s in R})
    crash = sum(1 for r in rows if r["cfg"] in ("def", B) and ("error" in r["result"] or st(r) in ("", "none", "crash", "error", "fetch_error")))
    full = [(p, s) for p in probs for s in seeds if all((p, c, s) in R for c in ("def", B))]
    nsol = {c: sum(solved(R[(p, c, s)]) for p, s in full) for c in ("def", B)}
    bad = []
    for p, s in full:
        objs = [R[(p, c, s)]["result"].get("objective") for c in ("def", B) if st(R[(p, c, s)]) == "optimal"]
        if objs and max(objs) - min(objs) > 1e-6 * max(1, abs(objs[0])): bad.append((p, s, objs))
        if {st(R[(p, c, s)]) for c in ("def", B)} >= {"optimal", "infeasible"}: bad.append((p, s, "opt/inf"))
    logr = collections.defaultdict(list)
    for p, s in full:
        a, b = R[(p, "def", s)], R[(p, B, s)]
        if solved(a) or solved(b): logr[p].append(math.log((t(b) + 1) / (t(a) + 1)))
    ps = list(logr)
    def sgm(q):
        v = [x for p in q for x in logr[p]]; return math.exp(sum(v) / len(v)) if v else 1.0
    random.seed(0); bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000)) if ps else [1.0] * 4000
    slow = [p for p in ps if len(logr[p]) >= 2 and all(x > math.log(1.25) for x in logr[p])]
    fast = [p for p in ps if len(logr[p]) >= 2 and all(x < math.log(0.8) for x in logr[p])]
    res = dict(S=S, B=B, rows=len(rows), crash=crash, pairs=len(full), nsol=nsol, bad=bad, n=len(ps),
               g=sgm(ps) - 1, lo=bs[100] - 1, hi=bs[3899] - 1, slow=slow, fast=fast)
    if verbose:
        print(f"\n## {S}: {B} / def (行数 {len(rows)}、問題 {len(probs)}、種 {seeds}、crash {crash}、そろった組 {len(full)})")
        print("解けた数:", nsol, " 食い違い", len(bad), bad)
        print(B, "最大 RSS", max([(r.get("max_rss_mb") or 0) for r in rows if r["cfg"] == B] or [0]), "MB / def",
              max([(r.get("max_rss_mb") or 0) for r in rows if r["cfg"] == "def"] or [0]), "MB")
        print(f"SGM (どちらかが解けた {len(ps)} 問): {res['g']:+.1%} [{res['lo']:+.1%}, {res['hi']:+.1%}]")
        print("全種で 1.25 倍を超えて遅い:", slow, " 全種で 0.8 倍未満:", fast)
        for p in sorted(ps, key=lambda q: -sum(logr[q]) / len(logr[q])):
            v = logr[p]; gg = math.exp(sum(v) / len(v))
            if gg > 1.15 or gg < 0.87: print(f"  {p}: {gg:.2f} {[round(math.exp(x), 2) for x in v]}")
    return res
def table(rs):
    print("\n== 採用条件")
    print("| 組 | 設定 | SGM 設定/def [95% CI] | CI 上限 | 解けた数 def → 設定 | 全種 1.25 倍超 | 食い違い | crash |")
    print("|---|---|---|---|---|---|---|---|")
    for r in rs:
        print(f"| {r['S']} | {r['B']} | {r['g']:+.1%} [{r['lo']:+.1%}, {r['hi']:+.1%}] | {r['hi']:+.1%} | {r['nsol']['def']} → {r['nsol'][r['B']]} | {', '.join(r['slow']) or 'なし'} | {len(r['bad'])} | {r['crash']} |")
    for B in BS:
        q = [r for r in rs if r["B"] == B]
        a, b = sum(r["nsol"]["def"] for r in q), sum(r["nsol"][B] for r in q)
        print(f"解けた数の合計 ({B}): def {a} → {B} {b} ({'減らない' if b >= a else '減る'})")
if ARG in ("PM", "PXMX"):
    table([summ(S, B, False) for S in (("P", "M") if ARG == "PM" else ("PX", "MX")) for B in BS])
else:
    table([summ(ARG, B) for B in BS])

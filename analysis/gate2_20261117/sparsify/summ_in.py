"""入る問題だけの比べ (README 7 節): python summ_in.py I J > summ_in.txt。res_<SET>_s*.jsonl を読み、問題ごとに種ごとの比 (spls/def、
時間は解けなければ制限時間、shift 1 秒)、解けた数、全種で 1.25 倍超の遅れ、時間切れのときの終わりのギャップ (|暫定解 − 下界| / max(1, |暫定解|))。"""
import json, glob, math, random, collections, sys
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for S in sys.argv[1:] for f in sorted(glob.glob(str(D / f"res_{S}_s*.jsonl"))) for l in open(f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
def st(r): return str(r["result"].get("status") or "").lower()
def solved(r): return st(r) in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
def gap(r):
    o, b = r["result"].get("objective"), r["result"].get("bound")
    return math.inf if o is None or b is None else abs(o - b) / max(1.0, abs(o))
probs = sorted({p for p, c, s in R})
crash = [(r["problem"], r["cfg"], r["seed"]) for r in rows if st(r) in ("", "crash", "error", "fetch_error")]
print(f"行数 {len(rows)}、問題 {len(probs)}、crash {len(crash)} {crash}")
bad = []; allr = []
for p in probs:
    seeds = sorted(s for s in {s for q, c, s in R if q == p} if (p, "def", s) in R and (p, "spls", s) in R)
    if not seeds: continue
    lr, gd, gs = [], [], []
    for s in seeds:
        a, b = R[(p, "def", s)], R[(p, "spls", s)]
        oa, ob = a["result"].get("objective"), b["result"].get("objective")
        if st(a) == "optimal" and st(b) == "optimal" and abs(oa - ob) > 1e-6 * max(1, abs(oa)): bad.append((p, s, oa, ob))
        if solved(a) and solved(b) is False and ob is not None and oa is not None and ob < oa - 1e-6 * max(1, abs(oa)): bad.append((p, s, "spls の暫定解が既定の最適値より良い", oa, ob))
        if solved(b) and solved(a) is False and oa is not None and ob is not None and oa < ob - 1e-6 * max(1, abs(ob)): bad.append((p, s, "既定の暫定解が spls の最適値より良い", oa, ob))
        lr.append(math.log((t(b) + 1) / (t(a) + 1))); gd.append(gap(a)); gs.append(gap(b))
    g = math.exp(sum(lr) / len(lr)); allr.append((p, lr))
    sa = sum(solved(R[(p, "def", s)]) for s in seeds); sb = sum(solved(R[(p, "spls", s)]) for s in seeds)
    tag = " 全種で 1.25 倍超の遅れ" if all(x > math.log(1.25) for x in lr) else (" 全種で速い" if all(x < math.log(0.8) for x in lr) else "")
    ta = [round(t(R[(p, "def", s)]), 1) for s in seeds]; tb = [round(t(R[(p, "spls", s)]), 1) for s in seeds]
    print(f"\n{p} (種 {seeds[0]}〜{seeds[-1]}、{R[(p, 'def', seeds[0])]['tl']} 秒): 比の SGM {g:.2f}、解けた {sa} → {sb}{tag}")
    print(f"  既定 {ta}\n  spls {tb}\n  比   {[round(math.exp(x), 2) for x in lr]}")
    if sa < len(seeds) or sb < len(seeds):
        fm = lambda v: [("inf" if x == math.inf else f"{x:.2e}") for x in v]
        print(f"  終わりのギャップ 既定 {fm(gd)}\n                 spls {fm(gs)}")
    rss = lambda c: max(R[(p, c, s)].get("max_rss_mb") or 0 for s in seeds)
    print(f"  最大 RSS {rss('def')} → {rss('spls')} MB")
if allr:
    ps = [p for p, _ in allr]; L = dict(allr)
    def sgm(q): v = [x for p in q for x in L[p]]; return math.exp(sum(v) / len(v))
    random.seed(0); bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000))
    print(f"\n入る問題の SGM (spls/def、問題で重みを等しく): {sgm(ps) - 1:+.1%} [{bs[100] - 1:+.1%}, {bs[3899] - 1:+.1%}] (問題 {len(ps)})")
print(f"食い違い・誤答の疑い {len(bad)} {bad}")

"""REVISIT の 58 問 (60 秒) の対の比較: 各設定を既定 (def) と同じ (問題, 種) で比べる。
SGM は shift 1 秒、時間切れは制限時間で数える。95% 区間は問題単位のブートストラップ (4000 回)。
使い方: python summ.py [res_P_s*.jsonl のグロブ (既定 res_P_s*.jsonl)]"""
import json, glob, math, random, collections, sys
from cfg import CFGS
pat = sys.argv[1] if len(sys.argv) > 1 else "res_P_s*.jsonl"
rows = [json.loads(l) for f in sorted(glob.glob(pat)) for l in open(f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
def ok(r): return r["result"].get("status") in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if ok(r) else r["tl"]
probs = sorted({k[0] for k in R}); seeds = sorted({k[2] for k in R})
print(f"行数 {len(rows)}、crash {sum(1 for r in rows if r['result'].get('status') in (None, 'crash', 'error', 'fetch_error'))}")
for c in CFGS:
    if c == "def": continue
    logr = collections.defaultdict(list); ns = [0, 0]; n = 0; bad = []; gain = []; lose = []
    for p in probs:
        for s in seeds:
            a, b = R.get((p, "def", s)), R.get((p, c, s))
            if not a or not b: continue
            n += 1; ns[0] += ok(a); ns[1] += ok(b)
            if ok(a) and not ok(b): lose.append(f"{p}#{s}")
            if ok(b) and not ok(a): gain.append(f"{p}#{s}")
            ra, rb = a["result"], b["result"]
            if ok(a) and ok(b):
                if ra["status"] != rb["status"]: bad.append((p, s, ra["status"], rb["status"]))
                elif ra["status"] == "optimal" and abs(ra["objective"] - rb["objective"]) > 1e-6 * max(1, abs(ra["objective"])): bad.append((p, s, ra["objective"], rb["objective"]))
            if ok(a) or ok(b): logr[p].append(math.log((t(b) + 1) / (t(a) + 1)))
    if not logr: continue
    ps = list(logr)
    def sgm(q):
        v = [x for p in q for x in logr[p]]; return math.exp(sum(v) / len(v))
    est = sgm(ps); random.seed(0)
    bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000))
    slow = [p for p in ps if len(logr[p]) >= 2 and len(logr[p]) == len(seeds) and all(x > math.log(1.25) for x in logr[p])]
    fast = [p for p in ps if len(logr[p]) >= 2 and len(logr[p]) == len(seeds) and all(x < math.log(0.8) for x in logr[p])]
    rss = [R[(p, c, s)]["max_rss_mb"] / max(1, R[(p, "def", s)]["max_rss_mb"]) for p in probs for s in seeds if (p, c, s) in R and (p, "def", s) in R and R[(p, "def", s)].get("max_rss_mb")]
    rr = math.exp(sum(math.log(max(x, 1e-3)) for x in rss) / len(rss)) if rss else float("nan")
    print(f"{c:7s} 対 {n:3d}  SGM {est-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}]  解けた {ns[0]}→{ns[1]}  得 {gain}  失 {lose}"
          f"  揃って遅い {slow}  揃って速い {fast}  RSS比 {rr:.2f}  食い違い {bad}")

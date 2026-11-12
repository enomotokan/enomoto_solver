"""PRUNE の leave-one-out の集計 (58 問 × 種 1・2、60 秒)。各設定 X を def と対にして比べる (X/def)。
SGM は shift 1 秒、時間切れは制限時間。CI は問題単位のブートストラップ (95%)。比 > 1 は「止めると遅い」= 部品が効いている。
使い方: python summ.py [設定 ...] → 標準出力 (summ.txt に保存する)"""
import json, glob, math, random, collections, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
from cfg import CFGS  # noqa: E402
rows = [json.loads(l) for f in sorted(glob.glob(str(D / "res_P_s*.jsonl"))) for l in open(f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
def solved(r): return r["result"].get("status") in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
def crashed(r): return r["result"].get("status") in (None, "crash", "error", "fetch_error") or "error" in r["result"]
probs = sorted({k[0] for k in R}); seeds = sorted({k[2] for k in R})
print(f"行数 {len(rows)}、問題 {len(probs)}、種 {seeds}、crash {sum(crashed(r) for r in rows)}")
print("設定 | 対 | 解けた def→X | SGM X/def [95%] | 全種で遅い (>1.25) | 全種で速い (<0.8) | 時間の和 def→X | 最大 RSS def→X | 食い違い")
for c in (sys.argv[1:] or [k for k in CFGS if k != "def"]):
    logr = collections.defaultdict(list); n = 0; na = nb = 0; bad = []; ta = tb = 0.0; ra = rb = 0
    for p in probs:
        for s in seeds:
            a, b = R.get((p, "def", s)), R.get((p, c, s))
            if not a or not b: continue
            n += 1; na += solved(a); nb += solved(b); ta += t(a); tb += t(b)
            ra = max(ra, a.get("max_rss_mb", 0)); rb = max(rb, b.get("max_rss_mb", 0))
            if a["result"].get("status") == "optimal" == b["result"].get("status"):
                oa, ob = a["result"]["objective"], b["result"]["objective"]
                if abs(oa - ob) > 1e-6 * max(1, abs(oa)): bad.append((p, s, oa, ob))
            if {a["result"].get("status"), b["result"].get("status")} == {"optimal", "infeasible"}: bad.append((p, s, "opt/inf"))
            if solved(a) or solved(b): logr[p].append(math.log((t(b) + 1) / (t(a) + 1)))
    ps = list(logr)
    if not ps: continue
    def sgm(q):
        v = [x for p in q for x in logr[p]]; return math.exp(sum(v) / len(v))
    random.seed(0); bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000))
    full = [p for p in ps if len(logr[p]) == len(seeds) and len(seeds) > 1]
    slow = [f"{p}{[round(math.exp(x), 2) for x in logr[p]]}" for p in full if all(x > math.log(1.25) for x in logr[p])]
    fast = [f"{p}{[round(math.exp(x), 2) for x in logr[p]]}" for p in full if all(x < math.log(0.8) for x in logr[p])]
    print(f"{c} | {n} | {na}→{nb} | {sgm(ps)-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}] | {' '.join(slow) or '-'} | {' '.join(fast) or '-'} | "
          f"{ta/3600:.2f}→{tb/3600:.2f} h | {ra}→{rb} | {bad or 0}")

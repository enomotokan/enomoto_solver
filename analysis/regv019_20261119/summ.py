"""案の計測の集計: python summ.py SET 案の設定 (例: python summ.py P vmf > summ_P_vmf.txt)。
案 / def の SGM (shift 10 秒、時間切れは上限)、問題単位のブートストラップの 95% CI (4000 回)、解けた数、全種で 1.25 倍超遅い・0.8 倍未満速い問題、
食い違い (最適値の違い・最適と実行不能の混在)、crash。採用の目安: CI 上端 ≲ +5%・解けた数が減らない・全種 1.25 倍超なし・誤答 0。"""
import json, glob, math, random, collections, sys
from pathlib import Path
D = Path(__file__).resolve().parent
S, cand = sys.argv[1], sys.argv[2]
CF = ["def", cand]
def st(r): return str(r["result"].get("status") or "").lower()
def solved(r): return st(r) in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
rows = [json.loads(l) for f in sorted(glob.glob(str(D / f"res_{S}_s*.jsonl"))) for l in open(f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
probs = sorted({p for p, c, s in R}); seeds = sorted({s for p, c, s in R})
crash = sum(1 for r in rows if st(r) in ("", "none", "crash", "error", "fetch_error"))
full = [(p, s) for p in probs for s in seeds if all((p, c, s) in R for c in CF)]
nsol = {c: sum(solved(R[(p, c, s)]) for p, s in full) for c in CF}
bad = []
for p, s in full:
    objs = [R[(p, c, s)]["result"].get("objective") for c in CF if st(R[(p, c, s)]) == "optimal"]
    if len(objs) == 2 and abs(objs[0] - objs[1]) > 1e-6 * max(1, abs(objs[0])): bad.append((p, s, objs))
    if {st(R[(p, c, s)]) for c in CF} >= {"optimal", "infeasible"}: bad.append((p, s, "opt/inf"))
logr = collections.defaultdict(list)
for p, s in full:
    a, b = R[(p, "def", s)], R[(p, cand, s)]
    if solved(a) or solved(b): logr[p].append(math.log((t(b) + 10) / (t(a) + 10)))
ps = list(logr)
def sgm(q):
    v = [x for p in q for x in logr[p]]; return math.exp(sum(v) / len(v)) if v else 1.0
random.seed(0); bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000)) if ps else [1.0] * 4000
print(f"{S}: 行数 {len(rows)}、問題 {len(probs)}、種 {seeds}、crash {crash}、そろった組 {len(full)}")
print("解けた数:", nsol, " 食い違い", len(bad), bad)
print(f"{cand} / def (どちらかが解けた {len(ps)} 問): {sgm(ps) - 1:+.1%} [{bs[100] - 1:+.1%}, {bs[3899] - 1:+.1%}]")
print("全種で 1.25 倍超遅い:", [p for p in ps if len(logr[p]) >= 2 and all(x > math.log(1.25) for x in logr[p])],
      " 全種で 0.8 倍未満:", [p for p in ps if len(logr[p]) >= 2 and all(x < math.log(0.8) for x in logr[p])])
for p in sorted(ps, key=lambda q: -sum(logr[q]) / len(logr[q])):
    g = math.exp(sum(logr[p]) / len(logr[p]))
    if g > 1.1 or g < 0.9: print(f"  {p}: {g:.2f} {[round(math.exp(x), 2) for x in logr[p]]}")

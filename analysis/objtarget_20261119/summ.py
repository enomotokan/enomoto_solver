"""OBJTARGET の l の計測の集計: python summ.py SET (ML か PL) > summ_SET.txt。
解けた数・時間の SGM (shift 1 秒、時間切れは上限、どちらかが解けた問題、問題単位のブートストラップ) と、
主のギャップの積分 (0〜上限秒、参照値 scripts/miplib2017-v31.solu、暫定解なしの間は 1) の比 (l / def、問題ごとの平均、SGM は shift 1)。"""
import json, glob, math, random, collections, sys
from pathlib import Path
D = Path(__file__).resolve().parent
SET = sys.argv[1]
ALT = sys.argv[2] if len(sys.argv) > 2 else "l"
ref = {}
for l in open(D.parents[1] / "scripts/miplib2017-v31.solu"):
    t = l.split()
    if len(t) >= 3 and t[0] in ("=opt=", "=best="): ref[t[1]] = float(t[2])
rows = [json.loads(l) for f in sorted(glob.glob(str(D / f"res_{SET}_s*.jsonl"))) for l in open(f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
def st(r): return str(r["result"].get("status") or "").lower()
def solved(r): return st(r) in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
def pg(o, z):
    if o is None: return 1.0
    if abs(o - z) <= 1e-9 * max(1.0, abs(z)): return 0.0
    if o * z < 0: return 1.0
    return abs(o - z) / max(abs(o), abs(z))

def shift_inc(r):
    """暫定解の推移の時刻 (木の時計) を外側の求解の時計に直す: 最初の暫定解の外側の時刻 (first_solution_time) との差を足す"""
    inc = r["result"].get("inc") or []
    fst = r["result"].get("first_solution_time")
    off = (fst - inc[0][0]) if (inc and fst is not None and fst >= inc[0][0]) else 0.0
    return [(t + off, v) for t, v in inc]
def pint(r):
    z = ref.get(r["problem"])
    if z is None: return None
    tl = r["tl"]; end = t(r); s = 0.0; t0 = 0.0; g = 1.0
    for ti, v in shift_inc(r):
        ti = min(ti, end); s += g * (ti - t0); t0 = ti; g = pg(v, z)
    s += g * (end - t0)
    return s
crash = sum(1 for r in rows if st(r) not in ("optimal", "infeasible", "time_limit"))
probs = sorted({p for p, c, s in R}); seeds = sorted({s for p, c, s in R})
full = [(p, s) for p in probs for s in seeds if (p, "def", s) in R and (p, ALT, s) in R]
print(f"{SET}: 行数 {len(rows)}、問題 {len(probs)}、種 {seeds}、crash {crash}、対 {len(full)}")
print("解けた数:", {c: sum(solved(R[(p, c, s)]) for p, s in full) for c in ("def", ALT)})
bad = [(p, s) for p, s in full if all(st(R[(p, c, s)]) == "optimal" for c in ("def", ALT)) and abs(R[(p, "def", s)]["result"]["objective"] - R[(p, ALT, s)]["result"]["objective"]) > 1e-6 * max(1, abs(R[(p, "def", s)]["result"]["objective"]))]
print("食い違い", bad)
def boot(logr):
    ps = list(logr)
    def g(q):
        v = [x for p in q for x in logr[p]]; return math.exp(sum(v) / len(v)) if v else 1.0
    random.seed(0); bs = sorted(g([random.choice(ps) for _ in ps]) for _ in range(4000))
    return g(ps), bs[100], bs[3899], len(ps)
lt = collections.defaultdict(list); li = collections.defaultdict(list); pis = {}
for p, s in full:
    a, b = R[(p, "def", s)], R[(p, ALT, s)]
    if solved(a) or solved(b): lt[p].append(math.log((t(b) + 1) / (t(a) + 1)))
    ia, ib = pint(a), pint(b)
    if ia is not None: li[p].append(math.log((ib + 1) / (ia + 1))); pis.setdefault(p, []).append((round(ia, 1), round(ib, 1)))
g, lo, hi, n = boot(lt); print(f"\n時間 l / def (どちらかが解けた {n} 問): {g-1:+.1%} [{lo-1:+.1%}, {hi-1:+.1%}]")
g, lo, hi, n = boot(li); print(f"主の積分 l / def ({n} 問、shift 1): {g-1:+.1%} [{lo-1:+.1%}, {hi-1:+.1%}]")
print("\n問題ごとの主の積分 (def, l) と比:")
for p in sorted(pis, key=lambda q: sum(li[q]) / len(li[q])):
    print(f"  {p}: {pis[p]} {math.exp(sum(li[p]) / len(li[p])):.2f}")

"""OBJTARGET: 最初の解の遅れを除いた主の積分の比: python summ3.py SET 試作。各組で、両方に暫定解が出た後 (遅い方の最初の解の時刻) から上限までの
主のギャップの積分の比 (試作 / def、shift 1、問題単位のブートストラップ)。片方にしか解がない組は除く。"""
import json, glob, math, random, collections, sys
from pathlib import Path
D = Path(__file__).resolve().parent
SET, ALT = sys.argv[1], sys.argv[2]
ref = {}
for l in open(D.parents[1] / "scripts/miplib2017-v31.solu"):
    t = l.split()
    if len(t) >= 3 and t[0] in ("=opt=", "=best="): ref[t[1]] = float(t[2])
R = {}
for f in sorted(g for g in glob.glob(str(D / f"res_{SET}_s*.jsonl")) if __import__("re").fullmatch(rf"res_{SET}_s\d+\.jsonl", Path(g).name)):
    for l in open(f):
        r = json.loads(l); R[(r["problem"], r["cfg"], r["seed"])] = r
def pg(o, z):
    if abs(o - z) <= 1e-9 * max(1.0, abs(z)): return 0.0
    if o * z < 0: return 1.0
    return abs(o - z) / max(abs(o), abs(z))
def end(r):
    st = str(r["result"].get("status")).lower()
    return min(r["result"]["time"], r["tl"]) if st in ("optimal", "infeasible") else r["tl"]

def shift_inc(r):
    """暫定解の推移の時刻 (外側の時計) を外側の求解の時計に直す: 最初の暫定解の外側の時刻 (first_solution_time) との差を足す"""
    inc = r["result"].get("inc") or []
    fst = r["result"].get("first_solution_time")
    off = (fst - inc[0][0]) if (inc and fst is not None and fst >= inc[0][0]) else 0.0
    return [(t + off, v) for t, v in inc]
def pint_from(r, z, t0):
    inc = shift_inc(r)
    g = None; s = 0.0; last = t0; te = end(r)
    for t, v in inc:
        if t <= t0: g = pg(v, z); continue
        t = min(t, te)
        if g is not None: s += g * (t - last)
        last = t; g = pg(v, z)
    if g is not None: s += g * max(0.0, te - last)
    return s
lr = collections.defaultdict(list); late = []
for (p, c, s), a in R.items():
    if c != "def" or (p, ALT, s) not in R or p not in ref: continue
    b = R[(p, ALT, s)]
    ia, ib = shift_inc(a), shift_inc(b)
    if not ia or not ib: continue
    t0 = max(ia[0][0], ib[0][0])
    pa, pb = pint_from(a, ref[p], t0), pint_from(b, ref[p], t0)
    lr[p].append(math.log((pb + 1) / (pa + 1)))
    late.append((p, s, round(ia[0][0], 1), round(ib[0][0], 1)))
ps = list(lr)
def g(q):
    v = [x for p in q for x in lr[p]]; return math.exp(sum(v) / len(v))
random.seed(0); bs = sorted(g([random.choice(ps) for _ in ps]) for _ in range(4000))
print(f"{SET}: 最初の解の後 (遅い方の最初の解から) の主の積分 {ALT} / def ({len(ps)} 問): {g(ps)-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}]")
d = [b - a for _, _, a, b in late]
print(f"{SET}: 最初の解の時刻 (外側の時計) の差 {ALT} − def: 中央値 {sorted(d)[len(d)//2]:+.1f} 秒、{ALT} が遅い組 {sum(x > 1 for x in d)}・早い組 {sum(x < -1 for x in d)} / {len(d)}")

"""木の探索が長くなる乱数の MIP (一般整数・0-1 の混じった多次元ナップサックに等式を足したもの) で、HiGHS と最適値を突き合わせる。
木の中の分離の試作 (ENOMOTO_MIP_TREESEP2) の検査に使う。使い方: python fuzz_tree.py 問題数 [開始seed]"""
import importlib.util, math, random, sys
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("fz", str(WT / "scripts/mip_fuzz_vs_highs.py"))
src = open(spec.origin).read().split("N=int(sys.argv[1])")[0]
ns = {}; exec(compile(src, spec.origin, "exec"), ns)
highs, ours = ns["highs"], ns["ours"]
inf = float("inf")
def gen(seed):
    g = random.Random(seed); n = g.randint(50, 100); m = g.randint(4, 10)
    lo = [0.0] * n
    it = [g.random() < 0.95 for _ in range(n)]
    up = [float(g.choice([1, 1, 1, 3, 5])) if it[j] else round(g.uniform(1, 10), 2) for j in range(n)]
    c = [-float(g.randint(20, 30)) for _ in range(n)]
    rows, rlo, rup = [], [], []
    for _ in range(m):
        js = g.sample(range(n), g.randint(10, min(30, n)))
        r = [(j, float(g.randint(20, 30))) for j in js]
        cap = sum(a * up[j] for j, a in r) * g.uniform(0.3, 0.6)
        rows.append(r); rlo.append(-inf); rup.append(round(cap))
    for _ in range(g.randint(0, 2)):
        js = g.sample([j for j in range(n) if it[j] and up[j] == 1.0] or [0], 1)
        ks = g.sample(range(n), g.randint(2, 5))
        rows.append([(j, 1.0) for j in set(js + ks)]); rlo.append(-inf); rup.append(float(g.randint(1, 3)))
    return lo, up, c, it, rows, rlo, rup
N = int(sys.argv[1]); start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
bad = 0
for s in range(start, start + N):
    lp = gen(s)
    hs, hz = highs(lp)[:2]
    os_, oz = ours(lp)[:2]
    ok = (hs == os_) or (str(hs).lower().startswith("opt") and os_ == "optimal")
    if str(hs).lower().startswith("opt") and os_ == "optimal":
        ok = abs(hz - oz) <= 1e-6 * max(1.0, abs(hz))
    if not ok:
        bad += 1
        print("MISMATCH", s, hs, hz, os_, oz, flush=True)
print(f"done {N} problems, mismatches {bad}")

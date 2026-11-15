"""fuzz_eqrst.py の食い違い (fz/*.txt の seed 行) を、許容誤差 1e-10・前処理なしの HiGHS (厳密に実行可能な最良の値) と比べて分ける。
使い方: python rejudge.py fz/X.txt ...  返した値が厳密な値より悪い (相対 1e-6) なら「誤り」、以下なら「許容誤差の内の差」。"""
import sys, re, io, contextlib
from pathlib import Path
sys.argv, files = [sys.argv[0], "0"], sys.argv[1:]
with contextlib.redirect_stdout(io.StringIO()):
    fp = str(Path(__file__).parent / "fuzz_eqrst.py"); ns = {"__name__": "x", "__file__": fp}; exec(compile(open(fp).read().split("N = int(sys.argv[1])")[0], fp, "exec"), ns)
spec = str(Path(__file__).resolve().parents[2] / "benchmarks/equiv2_bug_20261022/fuzz_tiny.py")
src = open(spec).read().split("N = int(sys.argv[1])")[0]
ft = {"__file__": spec}; exec(compile(src, spec, "exec"), ft)
for f in files:
    wrong = tol = 0; ws = []
    for l in open(f):
        m = re.match(r"seed (\d+) highs (\S+) (\S+) ours (\S+) (\S+)", l)
        if not m: continue
        seed, oo = int(m.group(1)), float(m.group(5))
        ts, to = ft["highs_tight"](ns["gen"](seed))
        if ts == "Optimal" and oo > to + 1e-6 * (1 + abs(to)): wrong += 1; ws.append((seed, oo, to))
        else: tol += 1
    print(f, "誤り", wrong, "許容誤差の内の差", tol, ws[:8])

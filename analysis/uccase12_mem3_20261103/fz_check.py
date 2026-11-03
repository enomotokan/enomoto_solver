"""fuzz_tiny の seed を 1 つ解き、ENOMOTO と HiGHS の解の違反 (行・境界・整数性) を出す。"""
import sys, runpy
sys.argv = ["fuzz_tiny.py", "0"]
src = open("benchmarks/equiv2_bug_20261022/fuzz_tiny.py").read().split("N = int(sys.argv[1])")[0]
ns = {"__file__": "benchmarks/equiv2_bug_20261022/fuzz_tiny.py"}; exec(compile(src, "fuzz_tiny.py", "exec"), ns)
import highspy, numpy as np
for seed in map(int, sys.argv_extra if hasattr(sys, "argv_extra") else [61, 81, 139, 148, 159, 195]):
    lp = ns["gen"](seed); lo, up, c, it, rows, rlo, rup = lp
    hs, ho = ns["highs"](lp); os_, oo, x = ns["ours"](lp)
    def viol(x):
        v = max(max(lo[j] - x[j], x[j] - up[j]) for j in range(len(x)))
        vi = max((abs(x[j] - round(x[j])) for j in range(len(x)) if it[j]), default=0)
        vr = max(max(rlo[i] - sum(a * x[j] for j, a in r), sum(a * x[j] for j, a in r) - rup[i]) for i, r in enumerate(rows))
        return v, vi, vr
    print(seed, "highs", ho, "ours", oo, "ours x viol (bound, int, row)", viol(x) if x is not None else None, "obj(x)", sum(ci * xi for ci, xi in zip(c, x)) if x is not None else None, flush=True)

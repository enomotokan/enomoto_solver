"""seed14 の例を縮める: 行にも目的にも出ない列を 1 本ずつ外し、誤答 (optimal を返す) が残るかを見る。python shrink.py"""
import math, os, sys, tempfile
from pathlib import Path
src = (Path(__file__).resolve().parents[2] / "objint_20261112" / "wrong_seed14.py").read_text()
ns = {"__file__": "x"}
exec(src.split("sys.path")[0] + src.split("import fuzz_objint as f")[1].split("print(")[0], ns)
LP = ns["LP"]
from enomoto_solver import Model, Variable
inf = math.inf

def solve(cols, lp=LP):
    lo, up, c, it, rows, rlo, rup, off, sense = lp
    M = Model(); xs = {j: Variable(int if it[j] else float, lo[j], up[j]) for j in cols}
    M.set_objective(sum(c[j] * xs[j] for j in cols) + off, sense)
    for r, l, u in zip(rows, rlo, rup):
        e = sum(v * xs[j] for j, v in r)
        if l > -inf: M.add_constraint(e >= l)
        if u < inf: M.add_constraint(e <= u)
    fd = os.open(os.devnull, os.O_WRONLY); s2 = os.dup(2); os.dup2(fd, 2)
    try: s = M.solve(mip_rel_gap=1e-9)
    finally: os.dup2(s2, 2)
    return s.status

used = {j for r in LP[4] for j, _ in r} | {j for j, v in enumerate(LP[2]) if v}
unused = [j for j in range(34) if j not in used]
cols = list(range(34))
print("all", solve(cols), "used only", solve(sorted(used)), "n used", len(used))
for j in unused:
    t = [k for k in cols if k != j]
    if solve(t) == "optimal":
        cols = t
print("minimal unused kept:", [j for j in cols if j in unused], solve(cols), "n", len(cols))

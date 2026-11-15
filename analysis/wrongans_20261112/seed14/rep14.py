"""analysis/objint_20261112/wrong_seed14.py の例を、ログ (stderr) を見せたまま解く。python rep14.py"""
import math, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "objint_20261112"))
import importlib.util
src = (Path(__file__).resolve().parents[2] / "objint_20261112" / "wrong_seed14.py").read_text()
ns = {"__file__": str(Path(__file__).resolve().parents[2] / "objint_20261112" / "wrong_seed14.py")}
exec(src.split("print(")[0], ns)
LP = ns["LP"]
from enomoto_solver import Model, Variable
inf = math.inf
lo, up, c, it, rows, rlo, rup, off, sense = LP
M = Model(); xs = [Variable(int if it[j] else float, lo[j], up[j]) for j in range(len(lo))]
M.set_objective(sum(cc * x for cc, x in zip(c, xs)) + off, sense)
for r, l, u in zip(rows, rlo, rup):
    e = sum(v * xs[j] for j, v in r)
    if l > -inf: M.add_constraint(e >= l)
    if u < inf: M.add_constraint(e <= u)
s = M.solve(mip_rel_gap=1e-9)
print("ENOMOTO", s.status, s.objective, [xs[j].value for j in (32, 33)])

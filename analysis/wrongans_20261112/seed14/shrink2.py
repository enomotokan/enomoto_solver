"""seed14 の例を行・項で縮める (返す点が実行不能な optimal が残るものだけ残す) (ENOMOTO_MIP_THREADS=1 で回す)。python shrink2.py"""
import math, os, sys
from pathlib import Path
src = (Path(__file__).resolve().parents[2] / "objint_20261112" / "wrong_seed14.py").read_text()
ns = {}
exec(src.split("sys.path")[0] + src.split("import fuzz_objint as f")[1].split("print(")[0], ns)
lo, up, c, it, rows, rlo, rup, off, sense = ns["LP"]
from enomoto_solver import Model, Variable
inf = math.inf

def solve(rs):
    cols = sorted({j for r, _, _ in rs for j, _ in r} | {j for j, v in enumerate(c) if v})
    M = Model(); xs = {j: Variable(int if it[j] else float, lo[j], up[j]) for j in cols}
    M.set_objective(sum(c[j] * xs[j] for j in cols) + off, sense)
    for r, l, u in rs:
        e = sum(v * xs[j] for j, v in r)
        if l > -inf: M.add_constraint(e >= l)
        if u < inf: M.add_constraint(e <= u)
    fd = os.open(os.devnull, os.O_WRONLY); s1 = os.dup(1); s2 = os.dup(2); os.dup2(fd, 2); os.dup2(fd, 1)
    try: s = M.solve(mip_rel_gap=1e-9)
    finally: os.dup2(s2, 2); os.dup2(s1, 1)
    if s.status != "optimal": return s.status
    x = {j: xs[j].value for j in cols}
    ok = all(lo[j] - 1e-6 <= x[j] <= up[j] + 1e-6 for j in cols) and all((l == -inf or sum(v * x[j] for j, v in r) >= l - 1e-6) and (u == inf or sum(v * x[j] for j, v in r) <= u + 1e-6) for r, l, u in rs)
    return "optimal" if not ok else "ok"

rs = [(list(r), l, u) for r, l, u in zip(rows, rlo, rup)]
print("start", solve(rs))
changed = True
while changed:
    changed = False
    for i in range(len(rs)):
        t = rs[:i] + rs[i + 1:]
        if t and solve(t) == "optimal":
            rs = t; changed = True; break
    if changed: continue
    for i in range(len(rs)):
        for k in range(len(rs[i][0])):
            r = rs[i][0][:k] + rs[i][0][k + 1:]
            if not r: continue
            t = rs[:i] + [(r, rs[i][1], rs[i][2])] + rs[i + 1:]
            if solve(t) == "optimal":
                rs = t; changed = True; break
        if changed: break
print(rs)
cols = sorted({j for r, _, _ in rs for j, _ in r} | {j for j, v in enumerate(c) if v})
for j in cols: print(j, lo[j], up[j], c[j], it[j])

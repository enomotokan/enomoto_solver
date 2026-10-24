"""ENOMOTO の tableau の候補の部分集合を LP 緩和に足して、どの選び方で上がるかを見る: python -I cutsub.py <mps> <cands_r0.txt> <h_x_r1.txt>"""
import sys, runpy
import numpy as np
mps, ec, hx = sys.argv[1:4]
g = runpy.run_path("cutcmp.py", init_globals={"LIBARGS": [mps, None, hx, ec, None]}, run_name="lib")
E, x, lp_with = g["E"], g["x"], g["lp_with"]
T = [c for c in E if c[0] == "tableau"]
def eff(c):
    _, r, cs = c; act = sum(a * x[j] for j, a in cs); nrm = np.sqrt(sum(a * a for _, a in cs)); return (act - r) / nrm
def viol(c):
    _, r, cs = c; return sum(a * x[j] for j, a in cs) - r
ef = np.array([eff(c) for c in T]); vi = np.array([viol(c) for c in T]); L = np.array([len(c[2]) for c in T])
print("tableau の候補", len(T))
for name, order in [("効き目の順", np.argsort(-ef)), ("違反量の順", np.argsort(-vi)), ("違反量/非零の順", np.argsort(-vi / L))]:
    for k in (50, 150, 500):
        sel = [T[i] for i in order[:k]]
        print(f"  {name} 上位 {k}: 非零の平均 {np.mean([len(c[2]) for c in sel]):.0f} -> LP {lp_with(sel)[0]:.6g}")
for cap in (20, 100, 400):
    sel = [c for c in T if len(c[2]) <= cap]
    print(f"  非零 {cap} 以下 {len(sel)} 本 -> LP {lp_with(sel)[0]:.6g}")
sel = [c for c in T if len(c[2]) > 400]
print(f"  非零 400 超 {len(sel)} 本 -> LP {lp_with(sel)[0]:.6g}")

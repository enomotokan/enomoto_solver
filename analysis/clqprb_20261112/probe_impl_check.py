"""CLQPRB: probe_impl.py の含意を伝播に使うと、HiGHS だけが固定した列の標本で x=1 が矛盾するかを確かめる。"""
import sys, json, pickle
sys.path.insert(0, ".")
from prop import load, propagate, row_cliques
P = load("mzzv11")
edges, fix = pickle.load(open("/tmp/claude-0/mzzv11_impl.pkl", "rb"))
rc = row_cliques(P)
for a, s in rc.items():
    edges.setdefault(a, set()).update(s)
lo0, up0 = P["lo"][:], P["up"][:]
for y, v in fix.items(): lo0[y] = up0[y] = float(v)
ok = propagate(P, lo0, up0, clq=edges)
print("base ok", ok, "fixed", sum(1 for k in range(P["n"]) if lo0[k] == up0[k] and P["lo"][k] != P["up"][k]))
for j in [3036, 8539, 1917, 4615, 2755, 8076, 7800, 8978, 2381, 8042, 947, 7314, 7701, 8788]:
    if lo0[j] == up0[j]: print(j, "already fixed", lo0[j]); continue
    lo, up = lo0[:], up0[:]; lo[j] = 1.0
    print(j, "x=1 ok:", propagate(P, lo, up, clq=edges))

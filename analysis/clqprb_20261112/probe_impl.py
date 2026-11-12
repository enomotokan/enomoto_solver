"""CLQPRB: mzzv11 の 0-1 列を全部、元の境界で x=0・x=1 にして行で伝播し (prop.py)、含意 (衝突の組) と固定を /tmp/claude-0/mzzv11_impl.pkl に書く (Python で約 8 分)。"""
import sys, json, time, pickle
sys.path.insert(0, ".")
from prop import load, propagate, row_cliques
P = load("mzzv11")
n = P["n"]
binj = [P["isint"][k] and P["lo"][k] == 0 and P["up"][k] == 1 for k in range(n)]
t0 = time.time()
edges = {}
fix = {}
def add(a, b):
    edges.setdefault(a, set()).add(b); edges.setdefault(b, set()).add(a)
for y in range(n):
    if not binj[y]: continue
    for v in (0, 1):
        lo, up = P["lo"][:], P["up"][:]
        if v: lo[y] = 1.0
        else: up[y] = 0.0
        ok = propagate(P, lo, up, queue_rows=[i for i, _ in P["cols"][y]])
        if not ok: fix[y] = 1 - v; continue
        for k in range(n):
            if k != y and binj[k] and lo[k] == up[k]:
                # (y=v) と (k = 1 - 値) が衝突
                add(2 * y + v, 2 * k + (1 - int(lo[k])))
print("time", time.time() - t0, "fixed", len(fix), "lits", len(edges), "edges", sum(len(s) for s in edges.values()) // 2)
pickle.dump((edges, fix), open("/tmp/claude-0/mzzv11_impl.pkl", "wb"))

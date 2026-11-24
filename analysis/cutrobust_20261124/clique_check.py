"""集合分割 (nw04 など) の LP 緩和の頂点で、clique・odd-cycle 型のカットが破れるかを確かめる: python clique_check.py 問題
HiGHS で LP 緩和を解き (前処理なし)、分数の列の衝突グラフ (同じ行を覆う 2 列は同時に 1 にできない) を作る。
- clique: 衝突グラフの極大 clique を全部挙げ (分数の列だけなので小さい)、Σ x* > 1 + 1e-6 で、どの元の行にも含まれないものを数える
- odd-cycle: 奇数長の閉路 C で Σ_{C} x* > (|C| − 1)/2 + 1e-6 となるものを、2 部の二重被覆の最短路 (重み 1 − x_i − x_j) で探す
  (Grötschel・Lovász・Schrijver の分離。重みの和 < 1 の奇閉路が破れる odd-cycle 不等式)"""
import sys, itertools, heapq, json
from pathlib import Path
import highspy, numpy as np
WT = Path(__file__).resolve().parents[2]
p = sys.argv[1]
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("presolve", "off")
h.readModel(str(WT / ".miplib_cache" / f"{p}.mps"))
lp = h.getLp()
# 整数性を外して LP 緩和
lp.integrality_ = []
h.passModel(lp); h.run()
x = np.array(h.getSolution().col_value); obj = h.getInfo().objective_function_value
A = lp.a_matrix_; ncol = lp.num_col_
# 列 → 行
start, index = np.array(A.start_), np.array(A.index_)
rows_of = [set(index[start[j]:start[j + 1]].tolist()) for j in range(ncol)]
F = [j for j in range(ncol) if 1e-6 < x[j] < 1 - 1e-6]
adj = {j: set() for j in F}
for a, b in itertools.combinations(F, 2):
    if rows_of[a] & rows_of[b]:
        adj[a].add(b); adj[b].add(a)
# 極大 clique (Bron–Kerbosch)
cl = []
def bk(R, P, X):
    if not P and not X: cl.append(R); return
    for v in list(P):
        bk(R | {v}, P & adj[v], X & adj[v]); P = P - {v}; X = X | {v}
bk(set(), set(F), set())
def in_row(C):
    common = set.intersection(*(rows_of[j] for j in C))
    return bool(common)
viol_cl = [(sorted(C), float(sum(x[j] for j in C))) for C in cl if len(C) >= 2 and sum(x[j] for j in C) > 1 + 1e-6 and not in_row(C)]
# odd-cycle: 二重被覆 (v, パリティ) の最短路
best = None
for s in F:
    dist = {(s, 0): 0.0}; pq = [(0.0, s, 0)]
    while pq:
        d, v, par = heapq.heappop(pq)
        if d > dist.get((v, par), 9e9) + 1e-12 or d >= 1 - 1e-6: continue
        for w in adj[v]:
            nd = d + max(0.0, 1 - x[v] - x[w]); key = (w, 1 - par)
            if nd < dist.get(key, 9e9) - 1e-12:
                dist[key] = nd; heapq.heappush(pq, (nd, w, 1 - par))
    d = dist.get((s, 1))
    if d is not None and d < 1 - 1e-6 and (best is None or d < best[0]): best = (d, s)
row = {"problem": p, "lp_obj": obj, "nfrac": len(F), "edges": sum(len(v) for v in adj.values()) // 2, "maximal_cliques": len(cl),
       "violated_cliques_not_in_a_row": len(viol_cl), "clique_examples": viol_cl[:3], "odd_cycle_min_weight": best[0] if best else None,
       "frac_values": sorted(round(float(x[j]), 4) for j in F)}
print(json.dumps(row))
with open(Path(__file__).resolve().parent / "clique_check.jsonl", "a") as f: f.write(json.dumps(row) + "\n")

"""HiGHS の Probing が消す列の手がかり: probe_gap.py <P0.mps (Probing を切った HiGHS の前処理後)> <P1.mps (既定の HiGHS の前処理後)>
P0 にあって P1 にない列 D について、P0 の行から作った衝突グラフ (CliqueGraph と同じ: 行の 2 文字を両方真にすると
行の最小活動が右辺を超える組) で
  fix   : 文字 v の隣に x と ~x が両方ある (v は偽)
  eq    : x と y が衝突し ~x と ~y も衝突する (x = 1 - y)、x と ~y・~x と y が衝突する (x = y)
  scc   : 衝突グラフの含意 (v -> ~w) の強連結成分で同値が導ける
を数える (行の伝播 1 段ではなく、衝突グラフの上の推論で見えるもの)。"""
import sys
from collections import defaultdict, Counter
sys.path.insert(0, __import__("os").path.dirname(__file__))
from twovar_census import load, scc  # noqa
from twovar_vs_highs import names  # noqa
import numpy as np

p0, p1 = sys.argv[1], sys.argv[2]
n, m, rows, rl, ru, binary, lo, up, integ = load(p0)
nm0 = names(p0)
gone = set(nm0) - set(names(p1))
D = [j for j in range(n) if nm0[j] in gone]
print("cols in P0 not in P1:", len(D), "binary", sum(binary[j] for j in D))
adj = defaultdict(set)
for i, r in enumerate(rows):
    for sg, b in ((1, ru[i]), (-1, -rl[i])):
        if not np.isfinite(b):
            continue
        minact = 0.0
        lits = []
        ok = True
        for j, a0 in r:
            a = sg * a0
            mm = a * lo[j] if a > 0 else a * up[j]
            if not np.isfinite(mm):
                ok = False
                break
            minact += mm
            if binary[j]:
                lits.append((abs(a), 2 * j if a > 0 else 2 * j + 1))
        if not ok or len(lits) < 2 or len(lits) > 2000:
            continue
        slack = b - minact
        lits.sort(reverse=True)
        for u in range(len(lits)):
            if u + 1 < len(lits) and lits[u][0] + lits[u + 1][0] <= slack + 1e-9:
                break
            for v in range(u + 1, len(lits)):
                if lits[u][0] + lits[v][0] <= slack + 1e-9:
                    break
                if lits[u][1] // 2 != lits[v][1] // 2:
                    adj[lits[u][1]].add(lits[v][1]); adj[lits[v][1]].add(lits[u][1])
print("conflict edges", sum(len(s) for s in adj.values()) // 2)
fixc = Counter(); eqc = Counter()
for j in D:
    if not binary[j]:
        continue
    for v in (2 * j, 2 * j + 1):
        nb = adj[v]
        if any((x ^ 1) in nb for x in nb):
            fixc["fix"] += 1
    nj, nnj = adj[2 * j], adj[2 * j + 1]
    if any((y ^ 1) in nnj for y in nj):
        eqc["eq"] += 1
dadj = [[] for _ in range(2 * n)]
for a, s in adj.items():
    for b in s:
        dadj[a].append(b ^ 1)
comp = scc(2 * n, dadj)
csz = Counter(comp)
inscc = sum(1 for j in D if binary[j] and csz[comp[2 * j]] >= 2)
contra = sum(1 for j in range(n) if binary[j] and comp[2 * j] == comp[2 * j + 1])
print("fix (literal adjacent to x and ~x):", fixc["fix"], " eq (pairwise):", eqc["eq"], " in nontrivial SCC:", inscc, " contradictory vars:", contra)
print("all binaries in nontrivial SCC:", sum(1 for j in range(n) if binary[j] and csz[comp[2*j]] >= 2))
deg = Counter(min(sum(1 for _ in ()) ,0) for _ in [])

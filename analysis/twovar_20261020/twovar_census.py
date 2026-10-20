"""2 変数の ±1 の不等式 (0-1 列 2 個) の内訳を数える: twovar_census.py <mps> [...]

各行を「文字の和 <= 1」(文字は x か 1-x) の形に直して、形で分ける:
  pack  : x + y <= 1          (文字 x, y)
  impl  : x - y <= 0 (x <= y) (文字 x, 1-y)
  cover : x + y >= 1          (文字 1-x, 1-y)
  other : それ以外 (右辺が違う・等式・範囲など)
さらに、2 変数の行だけから作った衝突グラフ (辺 = 2 文字の組) で
  tri2  : 辺が三角形に入る (2 変数の行だけのグラフで)
  triA  : 辺が三角形に入る (長いクリーク行 (文字の和 <= 1) も含めたグラフで)
  dom   : 辺の両端を含む長いクリーク行がすでにある (行は冗長)
  scc   : 含意の有向グラフ (辺 {a,b} は a -> ~b, b -> ~a) で a と ~b が同じ強連結成分 (= 同値が導ける)
  trans : 含意 a -> ~b が長さ 2 の別の道 a -> c -> ~b で導ける (推移的に冗長)
を数える。
"""
import sys
from collections import defaultdict, Counter

import highspy
import numpy as np

sys.setrecursionlimit(1 << 20)


def load(path):
    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    h.readModel(path)
    lp = h.getLp()
    n, m = lp.num_col_, lp.num_row_
    a = lp.a_matrix_
    start, index, value = np.array(a.start_), np.array(a.index_), np.array(a.value_)
    rows = [[] for _ in range(m)]
    if a.format_ == highspy.MatrixFormat.kColwise:
        for j in range(n):
            for k in range(start[j], start[j + 1]):
                rows[index[k]].append((j, value[k]))
    else:
        for i in range(m):
            for k in range(start[i], start[i + 1]):
                rows[i].append((index[k], value[k]))
    integ = list(lp.integrality_) if len(lp.integrality_) else [highspy.HighsVarType.kContinuous] * n
    lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
    binary = [integ[j] == highspy.HighsVarType.kInteger and lo[j] == 0 and up[j] == 1 for j in range(n)]
    return n, m, rows, np.array(lp.row_lower_), np.array(lp.row_upper_), binary, lo, up, integ


def lit(j, pos):
    return 2 * j if pos else 2 * j + 1


def clique_form(r, l, u, binary):
    """行が 0-1 列だけ・係数 ±1 で「文字の和 <= 1」の形なら文字のリスト (向きごと) を返す。"""
    out = []
    if not all(binary[j] and abs(abs(a) - 1) < 1e-12 for j, a in r):
        return out
    for sg, b in ((1.0, u), (-1.0, -l)):
        if not np.isfinite(b):
            continue
        nneg = sum(1 for _, a in r if sg * a < 0)
        if abs(b + nneg - 1) < 1e-9:
            out.append([lit(j, sg * a > 0) for j, a in r])
    return out


def scc(nv, adj):
    """反復版 Tarjan。成分番号を返す。"""
    index = [-1] * nv
    low = [0] * nv
    onst = [False] * nv
    comp = [-1] * nv
    st = []
    idx = 0
    c = 0
    for s in range(nv):
        if index[s] != -1:
            continue
        work = [(s, 0)]
        while work:
            v, pi = work.pop()
            if pi == 0:
                index[v] = low[v] = idx
                idx += 1
                st.append(v)
                onst[v] = True
            recurse = False
            nb = adj[v]
            while pi < len(nb):
                w = nb[pi]
                pi += 1
                if index[w] == -1:
                    work.append((v, pi))
                    work.append((w, 0))
                    recurse = True
                    break
                elif onst[w]:
                    low[v] = min(low[v], index[w])
            if recurse:
                continue
            if low[v] == index[v]:
                while True:
                    w = st.pop()
                    onst[w] = False
                    comp[w] = c
                    if w == v:
                        break
                c += 1
            if work:
                u = work[-1][0]
                low[u] = min(low[u], low[v])
    return comp


def census(path):
    n, m, rows, rl, ru, binary, lo, up, integ = load(path)
    nb = sum(binary)
    nint = sum(1 for t in integ if t == highspy.HighsVarType.kInteger)
    neq = int(sum(1 for i in range(m) if rl[i] == ru[i]))
    nnz = sum(len(r) for r in rows)
    shapes = Counter()
    edges = []  # (a, b, 形, 行)
    long_cliques = []
    for i, r in enumerate(rows):
        if len(r) == 2 and all(binary[j] and abs(abs(a) - 1) < 1e-12 for j, a in r):
            if rl[i] == ru[i]:
                shapes["eq"] += 1
                continue
            cf = clique_form(r, rl[i], ru[i], binary)
            if len(cf) != 1:
                shapes["other"] += 1
                continue
            a, b = cf[0]
            npos = (a % 2 == 0) + (b % 2 == 0)
            k = {2: "pack", 1: "impl", 0: "cover"}[npos]
            shapes[k] += 1
            edges.append((a, b, k, i))
        elif len(r) >= 3:
            for ls in clique_form(r, rl[i], ru[i], binary):
                long_cliques.append(ls)
    nv = 2 * n
    adj2 = defaultdict(set)
    for a, b, _, _ in edges:
        adj2[a].add(b)
        adj2[b].add(a)
    # 長いクリーク行の辺を加えたグラフ (隣接を全部作ると大きいので、文字 -> クリーク行の番号で持つ)
    by_lit = defaultdict(list)
    for c, ls in enumerate(long_cliques):
        for l in ls:
            by_lit[l].append(c)
    cl_sets = [set(ls) for ls in long_cliques]

    def nbrs_all(a):
        s = set(adj2[a])
        for c in by_lit.get(a, ()):
            s |= cl_sets[c]
        s.discard(a)
        return s

    # 含意の有向グラフ: 辺 {a,b} -> a => ~b, b => ~a
    neg = lambda l: l ^ 1
    dadj = [[] for _ in range(nv)]
    for a, b, _, _ in edges:
        dadj[a].append(neg(b))
        dadj[b].append(neg(a))
    comp = scc(nv, dadj)
    dset = [set(x) for x in dadj]
    stat = defaultdict(Counter)
    cache = {}
    for a, b, k, i in edges:
        s = stat[k]
        s["n"] += 1
        common2 = adj2[a] & adj2[b]
        if common2:
            s["tri2"] += 1
        na = cache.get(a)
        if na is None:
            na = cache[a] = nbrs_all(a)
        nbb = cache.get(b)
        if nbb is None:
            nbb = cache[b] = nbrs_all(b)
        if (na & nbb) - {a, b}:
            s["triA"] += 1
        if any(b in cl_sets[c] for c in by_lit.get(a, ())):
            s["dom"] += 1
        if comp[a] == comp[neg(b)]:
            s["scc"] += 1
        # a -> c -> ~b (c != ~b)
        if any(neg(b) in dset[c] for c in dadj[a] if c != neg(b)) or any(neg(a) in dset[c] for c in dadj[b] if c != neg(a)):
            s["trans"] += 1
    # 強連結成分 (大きさ 2 以上) の数と、それで消える列の数 (列 j と ~j が同じ成分なら矛盾 = 固定)
    csize = Counter(comp)
    nontriv = {c for c, z in csize.items() if z >= 2}
    vars_in = set()
    for l in range(nv):
        if comp[l] in nontriv:
            vars_in.add(l // 2)
    contra = sum(1 for j in range(n) if comp[2 * j] == comp[2 * j + 1])
    # 2 変数の行の辺の、貪欲な極大クリーク被覆 (2 変数の行だけのグラフで)
    covered = set()
    cliques = []
    order = sorted(adj2, key=lambda v: -len(adj2[v]))
    for v in order:
        for w in sorted(adj2[v], key=lambda x: -len(adj2[x])):
            e = (min(v, w), max(v, w))
            if e in covered:
                continue
            cl = [v, w]
            cand = adj2[v] & adj2[w]
            while cand:
                x = max(cand, key=lambda y: len(adj2[y] & cand))
                cl.append(x)
                cand &= adj2[x]
            for p in range(len(cl)):
                for q in range(p + 1, len(cl)):
                    covered.add((min(cl[p], cl[q]), max(cl[p], cl[q])))
            cliques.append(cl)
    big = [c for c in cliques if len(c) >= 3]
    edges_in_big = sum(len(c) * (len(c) - 1) // 2 for c in big)
    print(f"== {path}")
    print(f"rows {m} cols {n} (int {nint}, binary {nb}) eq {neq} nnz {nnz}")
    print(f"2-var +-1 binary rows: {dict(shapes)}  long clique rows (>=3 lits): {len(long_cliques)}")
    for k in ("pack", "impl", "cover"):
        s = stat[k]
        if s["n"]:
            print(f"  {k:5s} n={s['n']:6d} tri2={s['tri2']:6d} triA={s['triA']:6d} dom={s['dom']:6d} scc={s['scc']:6d} trans={s['trans']:6d}")
    print(f"  implication SCC: nontrivial comps {len(nontriv)} covering {len(vars_in)} vars, contradictory vars {contra}")
    print(f"  greedy clique cover of 2-var edges: {len(cliques)} cliques, {len(big)} of size>=3 (sizes {Counter(len(c) for c in big).most_common(8)}), covering {edges_in_big} edge-slots")
    sys.stdout.flush()


if __name__ == "__main__":
    for p in sys.argv[1:]:
        census(p)

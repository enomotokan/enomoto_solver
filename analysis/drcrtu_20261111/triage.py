"""TU の判定に落ちた行列 (count.py の dump) の形を集め、短い「悪い閉路」(Camion: 弦のない閉路で (k − 負の数) が奇数 → 行列式 ±2、TU でない)
を探して、確かに TU でないものを数える。python triage.py <dump のディレクトリ> > 出力"""
import sys, os, collections, itertools
def load(p):
    with open(p) as f:
        nr, nc, nn = map(int, f.readline().split())
        cols = []
        for line in f:
            c = {}
            for t in line.split():
                i, a = t.split(":"); c[int(i)] = float(a)
            cols.append(c)
    return nr, nc, nn, cols
def peel(nr, cols):
    rows = collections.defaultdict(dict)
    for j, c in enumerate(cols):
        for i, a in c.items(): rows[i][j] = a
    ra = set(rows); ca = set(range(len(cols)))
    ch = True
    while ch:
        ch = False
        for i in list(ra):
            if sum(1 for j in rows[i] if j in ca) <= 1: ra.discard(i); ch = True
        for j in list(ca):
            if sum(1 for i in cols[j] if i in ra) <= 1: ca.discard(j); ch = True
    return ra, ca, rows
def bad_cycle(cols, rows, ra, ca, new, maxk=3):
    # 新しい列を含む弦のない閉路 (長さ 2k、k ≤ maxk) で (k − 負の数) が奇数のもの
    for j0 in new:
        if j0 not in ca: continue
        # 列 j0 から始めて 行 → 列 → … と辿る DFS
        def dfs(path_c, path_r, neg):
            k = len(path_c)
            jl = path_c[-1]
            for i in cols[jl]:
                if i not in ra or i in path_r: continue
                a = cols[jl][i]
                # 閉じる: 行 i が j0 にも入っていれば
                if k >= 2 and j0 in rows[i] and len(path_r) == k - 1:
                    rs = path_r + [i]
                    # 弦なし: 部分行列の非零がちょうど 2k
                    nz = sum(1 for r in rs for c in path_c if c in rows[r])
                    if nz == 2 * k:
                        ng = neg + (a < 0) + (rows[i][j0] < 0)
                        if (k - ng) % 2 == 1: return True
                if k >= maxk: continue
                for j in rows[i]:
                    if j not in ca or j in path_c or j == j0: continue
                    if dfs(path_c + [j], path_r + [i], neg + (a < 0) + (rows[i][j] < 0)): return True
            return False
        if dfs([j0], [], 0): return True
    return False
d = sys.argv[1]
agg = collections.Counter()
for fn in sorted(os.listdir(d)):
    nr, nc, nn, cols = load(os.path.join(d, fn))
    nnz = sum(len(c) for c in cols)
    ra, ca, rows = peel(nr, cols)
    new = list(range(nc - nn, nc))
    cnz = collections.Counter(min(len(cols[j]), 9) for j in new)
    allpm = all(abs(a) == 1 for c in cols for a in c.values())
    small = nnz <= 20000
    bad = bad_cycle(cols, rows, ra, ca, new, 3 if small else 2) if small or nnz < 200000 else None
    print(f"{fn} rows {nr} cols {nc} new {nn} nnz {nnz} core {len(ra)}x{len(ca)} newcolnz {dict(sorted(cnz.items()))} pm1 {allpm} badcycle {bad}", flush=True)

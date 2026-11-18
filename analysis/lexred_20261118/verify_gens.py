"""DEBUG_DUMP_SYMGRAPH の書き出し (問題と生成元) を読み、各生成元が問題の自己同型 (費用・境界・整数性・行の集合を保つ) かを確かめる。
python verify_gens.py 書き出し.txt"""
import struct, sys
def f(h): return struct.unpack(">d", bytes.fromhex(h))[0] if h != "0000000000000000" else 0.0
L = open(sys.argv[1]).read().split("\n")
n, m = map(int, L[0].split()[:2])
cols, rows, gens = [], [], []
for l in L[1:]:
    t = l.split()
    if not t: continue
    if t[0] == "C": cols.append((f(t[1]), f(t[2]), f(t[3]), t[4]))
    elif t[0] == "R":
        k = int(t[3]); rows.append((f(t[1]), f(t[2]), tuple(sorted((int(t[4 + 2 * i]), f(t[5 + 2 * i])) for i in range(k)))))
    elif t[0] == "G":
        g = list(range(n)); a = t[1:]
        for i in range(0, len(a), 2): g[int(a[i])] = int(a[i + 1])
        gens.append(g)
rowset = {}
for r in rows: rowset[r] = rowset.get(r, 0) + 1
bad = 0
for k, g in enumerate(gens):
    ok = all(cols[j] == cols[g[j]] for j in range(n))
    img = {}
    for lo, up, r in rows:
        rr = (lo, up, tuple(sorted((g[j], a) for j, a in r)))
        img[rr] = img.get(rr, 0) + 1
    ok = ok and img == rowset
    if not ok:
        bad += 1; print("NOT a symmetry: gen", k, [(j, g[j]) for j in range(n) if g[j] != j])
print("gens", len(gens), "bad", bad)

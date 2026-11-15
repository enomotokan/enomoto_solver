"""ENOMOTO_DEBUG_PRESOLVE_DUMP の 2 つ (並べ替え 0 と k) を、列の名前で並べ替えを戻して段ごとに突き合わせる:
python dumpcmp.py 元.mps 並べ替えた.mps dump0 dumpk [呼び出し数]"""
import random, sys
import highspy
src, srck, f0, fk = sys.argv[1:5]
ncall = int(sys.argv[5]) if len(sys.argv) > 5 else 1
def names(f):
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(f); return list(h.getLp().col_names_)
N0, Nk = names(src), names(srck); n = len(N0); pos0 = {x: j for j, x in enumerate(N0)}
cp = [pos0[x] for x in Nk]  # 並べ替えた列 j は元の列 cp[j]
def blocks(f):
    out = []; cur = []
    for l in open(f):
        r, lab, v = l.rstrip("\n").split("\t")
        b = [tuple(map(float, x.split(","))) for x in v.split(";") if x]
        cur.append((r, lab, b))
        if lab == "final": out.append(cur); cur = []
    return out
B0, Bk = blocks(f0), blocks(fk)
for c in range(min(ncall, len(B0), len(Bk))):
    for (r0, l0, b0), (rk, lk, bk) in zip(B0[c], Bk[c]):
        if len(b0) != n or len(bk) != n: print("size", len(b0), len(bk)); break
        bk0 = [None] * n
        for j in range(n): bk0[cp[j]] = bk[j]
        d = [j for j in range(n) if b0[j] != bk0[j]]
        fx0 = sum(1 for x in b0 if x[0] == x[1]); fxk = sum(1 for x in bk if x[0] == x[1])
        print(c, r0, l0, (rk, lk) if (r0, l0) != (rk, lk) else "", "fixed", fx0, fxk, "diff", len(d), [(j, b0[j], bk0[j]) for j in d[:3]])
        if (r0, l0) != (rk, lk): break

"""addpairs.py <土台の mps (列名 C<番号>)> <出力 mps> <元の mps> <HiGHS の .subst> <ENOMOTO の .pairs> <形: highs|eno|enoonly>
土台に同値の組の等式 (x_a - x_b = 0 / x_a + x_b = 1) を足す。highs: HiGHS の 732 組、eno: ENOMOTO の全部、enoonly: ENOMOTO にしかない組だけ"""
import sys, highspy, numpy as np
base, out, orig, subst, pairs, mode = sys.argv[1:7]
h0 = highspy.Highs(); h0.setOptionValue('output_flag', False); h0.readModel(orig); names = list(h0.getLp().col_names_); idx = {n: j for j, n in enumerate(names)}
hp = []
for l in open(subst):
    if l.startswith('#'): continue
    a, b, v, *_ = l.split(); hp.append((idx[a], idx[b], 1 - int(v)))
ep = [tuple(map(int, l.split())) for l in open(pairs)]
exec(open('paircmp_uf.py').read())
H = UF()
for a, b, n in hp: H.union(a, b, n)
P = {'highs': hp, 'eno': ep, 'enoonly': [p for p in ep if H.rel(p[0], p[1]) is None]}[mode]
h = highspy.Highs(); h.setOptionValue('output_flag', False); h.readModel(base); lp = h.getLp()
col = {n: j for j, n in enumerate(lp.col_names_)}
k = 0
for a, b, n in P:
    ia, ib = col[f'C{a}'], col[f'C{b}']
    r = 1.0 if n else 0.0
    h.addRow(r, r, 2, np.array([ia, ib], dtype=np.int32), np.array([1.0, 1.0 if n else -1.0]))
    k += 1
h.writeModel(out); print(f"{mode}: {k} rows added -> {out}")

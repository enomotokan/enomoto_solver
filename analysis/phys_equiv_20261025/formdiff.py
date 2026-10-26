"""formdiff.py <元の mps> <形 A の mps> <形 B の mps>: 二つの前処理の後の形を元の列名で突き合わせる。
ENOMOTO の書き出し (列名 C<番号>) は元の列名に直す。自由な列の集まり・行の種類 (集合詰め込み・等式・その他) と、
支えの同じ行がもう一方にあるか (係数・右辺の違いも) を数える。"""
import sys, collections, highspy, numpy as np
def load(f, names):
    h = highspy.Highs(); h.setOptionValue('output_flag', False); h.readModel(f); lp = h.getLp()
    cn = list(lp.col_names_)
    if cn and cn[0].startswith('C') and cn[0][1:].isdigit(): cn = [names[int(c[1:])] for c in cn]
    lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
    free = {cn[j] for j in range(lp.num_col_) if lo[j] < up[j]}
    fixed = {cn[j]: lo[j] for j in range(lp.num_col_) if lo[j] == up[j]}
    A = lp.a_matrix_; st, ix, vl = np.array(A.start_), np.array(A.index_), np.array(A.value_)
    rows = collections.defaultdict(list)
    for j in range(lp.num_col_):
        if lo[j] == up[j]: continue
        for k in range(st[j], st[j + 1]): rows[ix[k]].append((cn[j], vl[k]))
    # 固定の列の分を右辺に移す
    shift = collections.defaultdict(float)
    for j in range(lp.num_col_):
        if lo[j] == up[j] and lo[j] != 0:
            for k in range(st[j], st[j + 1]): shift[ix[k]] += vl[k] * lo[j]
    R = []
    for i in range(lp.num_row_):
        r = sorted(rows.get(i, []))
        if not r: continue
        R.append((tuple(r), lp.row_lower_[i] - shift[i], lp.row_upper_[i] - shift[i]))
    integ = list(lp.integrality_) if len(lp.integrality_) else []
    return free, fixed, R, lp.offset_
def kind(r, free_bin):
    sup, lo, up = r
    if all(abs(abs(a) - 1) < 1e-12 and c in free_bin for c, a in sup):
        nneg = sum(1 for c, a in sup if a < 0)
        if abs(up + nneg - 1) < 1e-9 and lo < -1e20: return 'sp<=' if len(sup) > 2 else 'sp2<='
        if abs(-lo + (len(sup) - nneg) - 1) < 1e-9 and up > 1e20: return 'sp>='
        if lo == up: return 'eq01'
    if lo == up: return 'eq'
    return 'other'
h0 = highspy.Highs(); h0.setOptionValue('output_flag', False); h0.readModel(sys.argv[1]); names = list(h0.getLp().col_names_)
FA = load(sys.argv[2], names); FB = load(sys.argv[3], names)
# 同値の類 (ENOMOTO の組 + HiGHS の組) の代表に列を写す (どちらの列を残したかの違いを消す。補の向きは見ない)
if len(sys.argv) > 5:
    exec(open('paircmp_uf.py').read()); U = UF(); idx = {n: j for j, n in enumerate(names)}
    for l in open(sys.argv[4]):
        if l.startswith('#'): continue
        a, b, v, *_ = l.split(); U.union(idx[a], idx[b], 1 - int(v))
    for l in open(sys.argv[5]):
        j, k, ng = map(int, l.split()); U.union(j, k, ng)
    rep = lambda c: names[U.find(idx[c])[0]] if idx[c] in U.p else c
    def remap(F):
        free, fixed, R, off = F
        return ({rep(c) for c in free}, fixed, [(tuple(sorted((rep(c), a) for c, a in r[0])), r[1], r[2]) for r in R], off)
    FA, FB = remap(FA), remap(FB)
for nm, (free, fixed, R, off) in (('A', FA), ('B', FB)):
    kinds = collections.Counter(kind(r, free) for r in R)
    print(f"{nm}: 自由な列 {len(free)}、行 {len(R)}、非零 {sum(len(r[0]) for r in R)}、種類 {dict(kinds)}")
fa, fb = FA[0], FB[0]
print(f"自由な列: 両方 {len(fa & fb)}、A だけ {len(fa - fb)}、B だけ {len(fb - fa)}")
def key(r): return frozenset(c for c, a in r[0])
SA = {key(r): r for r in FA[2]}; SB = {key(r): r for r in FB[2]}
print(f"支えの同じ行: {len(set(SA) & set(SB))}、A だけ {len(set(SA) - set(SB))}、B だけ {len(set(SB) - set(SA))}")
# A だけの行が B の行の部分集合か上位集合か
def sub_super(S1, S2):
    by = collections.defaultdict(list)
    for k in S2: 
        for c in k: by[c].append(k)
    sub = sup = 0
    for k in S1:
        if k in S2: continue
        c0 = min(k, key=lambda c: len(by[c]))
        cand = by[c0]
        if any(k < k2 for k2 in cand): sub += 1
        elif any(k2 < k for k2 in cand): sup += 1
    return sub, sup
print("A だけの行のうち B の行に含まれる / B の行を含む:", sub_super(set(SA), set(SB)))
print("B だけの行のうち A の行に含まれる / A の行を含む:", sub_super(set(SB), set(SA)))
ka = collections.Counter(kind(SA[k], fa) + str(min(len(k), 9)) for k in set(SA) - set(SB))
kb = collections.Counter(kind(SB[k], fb) + str(min(len(k), 9)) for k in set(SB) - set(SA))
print("A だけの行の種類と長さ:", ka.most_common(12)); print("B だけの行の種類と長さ:", kb.most_common(12))

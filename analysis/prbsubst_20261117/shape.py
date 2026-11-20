"""MPS の形を数える (行: 等式/不等式・長さ・列の種類の混ざり方、列: 0-1/一般の整数/連続)。H115 の python で:
python shape.py a.mps [b.mps ...]"""
import sys, collections, highspy
for f in sys.argv[1:]:
    h = highspy.Highs(); h.setOptionValue('output_flag', False); h.readModel(f); lp = h.getLp()
    n, m = lp.num_col_, lp.num_row_
    st = list(lp.a_matrix_.start_); ix = list(lp.a_matrix_.index_); va = list(lp.a_matrix_.value_)
    lo = list(lp.col_lower_); up = list(lp.col_upper_); it = [t == highspy.HighsVarType.kInteger for t in lp.integrality_] if lp.integrality_ else [False] * n
    kind = ['b' if it[j] and lo[j] == 0 and up[j] == 1 else ('i' if it[j] else 'c') for j in range(n)]
    rows = collections.defaultdict(list)
    for j in range(n):
        for k in range(st[j], st[j + 1]): rows[ix[k]].append((j, va[k]))
    rl = list(lp.row_lower_); ru = list(lp.row_upper_)
    kc = collections.Counter(kind)
    cnt = collections.Counter()
    for i in range(m):
        r = rows[i]; eq = rl[i] == ru[i]
        ks = ''.join(sorted(set(kind[j] for j, _ in r)))
        size = len(r); sz = str(size) if size <= 4 else ('5-9' if size < 10 else '10+')
        unit = all(abs(v) == 1 for _, v in r)
        cnt[(('eq' if eq else 'in'), ks, sz, 'unit' if unit else 'gen')] += 1
    print(f"== {f}: {m} 行 {n} 列 列の種類 {dict(kc)}")
    for k, v in cnt.most_common(25): print("  ", k, v)

import highspy, sys, json, collections, itertools
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(sys.argv[1])
lp = h.getLp(); n = lp.num_col_
A = lp.a_matrix_; st, ix, va = list(A.start_), list(A.index_), list(A.value_)
rows = collections.defaultdict(dict)
colwise = A.format_ == highspy.MatrixFormat.kColwise
print("format", A.format_, "nnz", len(va), "cols", n)
for j in range(len(st) - 1):
    for k in range(st[j], st[j+1]):
        if colwise: rows[ix[k]][j] = va[k]
        else: rows[j][ix[k]] = va[k]
rlo, rup = list(lp.row_lower_), list(lp.row_upper_)
names = list(lp.col_names_)
data = {i: r for i, r in rows.items() if 0 in r and len(r) >= 6}
print("example", list(rows.items())[5][1] if len(rows)>5 else None, names[:3])
print("data rows with x0:", len(data), "of", len(rows))
d = json.load(open(sys.argv[2]))
cuts = d["pools"][0]["examples"]
# 列名 -> 番号
pos = {nm: j for j, nm in enumerate(names)}
# z_j = col j+113 ↔ w_j
for cut in cuts[:8]:
    terms, lo_, up_ = cut
    zs = sorted(pos[t[0]] for t in terms if t[1] == "B"); ws = sorted(pos[t[0]] for t in terms if t[1] != "B")
    target = set(z - 113 for z in zs)
    # 2 行の和 (x0 の符号が逆) で、残る w の集合が target (と ws) に含まれるもの
    allow = target | set(ws)
    keys = {}
    for i, r in data.items():
        if r[0] > 0:
            keys.setdefault(tuple(sorted((j, round(v, 9)) for j, v in r.items() if j not in allow)), []).append(i)
    found = None
    for i2, r2 in data.items():
        if r2[0] < 0:
            k = tuple(sorted((j, round(-v, 9)) for j, v in r2.items() if j not in allow))
            if k in keys:
                i1 = keys[k][0]; s_ = collections.Counter(data[i1]); s_.update(r2)
                found = (i1, i2, sorted((j, v) for j, v in s_.items() if abs(v) > 1e-9), rup[i1] + rup[i2]); break
    print(len(zs), "z", len(ws), "w", "rhs", lo_, up_, "target", sorted(target), "pair:", found)

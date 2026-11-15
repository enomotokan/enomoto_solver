import highspy, sys, collections
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(sys.argv[1]); h.presolve()
lp = h.getPresolvedLp()
n, m = lp.num_col_, lp.num_row_
lo, up, it = list(lp.col_lower_), list(lp.col_upper_), list(lp.integrality_)
A = lp.a_matrix_; st, ix, va = list(A.start_), list(A.index_), list(A.value_)
rows = collections.defaultdict(list)
for j in range(n):
    for k in range(st[j], st[j+1]): rows[ix[k]].append((j, va[k]))
kind = lambda j: ("I" if int(it[j]) == 1 else "C") if not (int(it[j]) == 1 and lo[j] == 0 and up[j] == 1) else "B"
print("bounds of non-binary ints:", collections.Counter((lo[j], up[j]) for j in range(n) if kind(j) == "I").most_common(6))
print("bounds of cont:", collections.Counter((lo[j], up[j]) for j in range(n) if kind(j) == "C").most_common(6))
sig = collections.Counter()
for i, r in rows.items():
    sig[tuple(sorted(collections.Counter(kind(j) for j, _ in r).items()))] += 1
print(sig.most_common(12))
big = [j for j in range(n) if kind(j) == "I" and up[j] - lo[j] > 50]
print("wide ints", [(j, lo[j], up[j], sum(1 for r in rows.values() if any(c == j for c, _ in r))) for j in big])

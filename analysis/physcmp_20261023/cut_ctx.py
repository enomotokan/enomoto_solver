"""カットの列を含む行 (基の行だけ) と LP 解の値を表示: python cut_ctx.py <cutlp mps> <基の行数> <pool json> <カット番号> ..."""
import json, sys
import numpy as np, highspy
mps, nbase, pj = sys.argv[1], int(sys.argv[2]), sys.argv[3]
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(mps); lp = h.getLp()
h.run(); x = np.array(h.getSolution().col_value)
n, m = lp.num_col_, lp.num_row_
A = lp.a_matrix_; st, ix, vl = np.array(A.start_), np.array(A.index_), np.array(A.value_)
rows = [[] for _ in range(m)]
for j in range(n):
    for p in range(st[j], st[j + 1]): rows[ix[p]].append((j, vl[p]))
rl, ru = np.array(lp.row_lower_), np.array(lp.row_upper_)
cost = np.array(lp.col_cost_); lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
P = json.load(open(pj)); S, I, V = P["start"], P["index"], P["value"]
for c in map(int, sys.argv[4:]):
    idx = I[S[c]:S[c + 1]]; val = V[S[c]:S[c + 1]]
    print(f"cut {c}: {P['lower'][c]:g} <= " + " ".join(f"{v:+g}c{j}({x[j]:.2f})" for j, v in zip(idx, val)) + f" <= {P['upper'][c]:g}")
    S2 = set(idx)
    for i in range(nbase):
        r = rows[i]
        if any(j in S2 for j, _ in r) and len(r) <= 30:
            print(f"   r{i}: {rl[i]:g} <= " + " ".join(f"{v:+g}c{j}({x[j]:.2f}){'$'+format(cost[j],'g') if cost[j] else ''}" for j, v in r) + f" <= {ru[i]:g}")

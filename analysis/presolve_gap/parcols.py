# 前処理後の問題で平行な列 (係数が定数倍) の組を数える。目的の係数も同じ倍なら「完全に平行」
import sys, highspy, numpy as np, collections
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(sys.argv[1]); lp = h.getLp()
st = np.array(lp.a_matrix_.start_); idx = np.array(lp.a_matrix_.index_); val = np.array(lp.a_matrix_.value_)
lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_); c = np.array(lp.col_cost_)
integ = np.array([int(v) for v in lp.integrality_])
buckets = collections.defaultdict(list)
for j in range(lp.num_col_):
    if lo[j] == up[j] or st[j] == st[j+1]: continue
    ii = idx[st[j]:st[j+1]]; vv = val[st[j]:st[j+1]]; o = np.argsort(ii); ii, vv = ii[o], vv[o]
    s = vv[0]
    buckets[(tuple(ii), tuple(np.round(vv / s, 9)))].append((j, s))
kinds = collections.Counter()
for k, cols in buckets.items():
    if len(cols) < 2: continue
    for (j, s) in cols[1:]:
        j0, s0 = cols[0]
        ratio = s / s0
        objpar = abs(c[j] - ratio * c[j0]) <= 1e-9 * max(1, abs(c[j]))
        kinds[(int(integ[j0]), int(integ[j]), round(ratio, 3), objpar, (lo[j0], up[j0]) == (lo[j], up[j]))] += 1
print(sum(kinds.values()), kinds.most_common(12))

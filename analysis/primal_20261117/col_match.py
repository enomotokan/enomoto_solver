"""調べ C の 15 節: HiGHS 1.16 の前処理の後の問題と元の問題 (sat_rens) の列を名前で突き合わせ、プロービングで消えた列の種類を数える。
集約 (規則 12) と列挙 (16) を止めた上で、プロービング (15) あり・なしの差を見る。python col_match.py MPS"""
import sys, collections, highspy
mps = sys.argv[1]
def presolved(off):
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("threads", 1); h.setOptionValue("presolve_rule_off", off)
    h.readModel(mps); lp0 = h.getLp(); h.presolve(); lp = h.getPresolvedLp()
    return lp0, lp
lp0, a = presolved((1 << 12) | (1 << 16))            # プロービングあり
_, b = presolved((1 << 12) | (1 << 16) | (1 << 15))   # プロービングなし
names0 = list(lp0.col_names_)
it = list(lp0.integrality_); lo = list(lp0.col_lower_); up = list(lp0.col_upper_)
kind = {}
for j, nm in enumerate(names0):
    if lo[j] == up[j]: k = "固定済み"
    elif int(it[j]) == 1 and lo[j] >= 0 and up[j] <= 1: k = "2 値"
    elif int(it[j]) == 1: k = "一般整数"
    else: k = "連続"
    kind[nm] = k
sa, sb = set(a.col_names_), set(b.col_names_)
print("元の列", len(names0), "種類", collections.Counter(kind.values()))
print("プロービングなし (12・15・16 を止める):", len(sb), "列", collections.Counter(kind[x] for x in sb))
print("プロービングあり (12・16 を止める):  ", len(sa), "列", collections.Counter(kind[x] for x in sa))
gone = sb - sa
print("プロービングで消えた列:", len(gone), collections.Counter(kind[x] for x in gone))
print("プロービングありだけに残る列 (代入で名前が残る側など):", len(sa - sb))
# 消えた 2 値列の行の数の分布 (元の問題での非零の数)
st = list(lp0.a_matrix_.start_); deg = {names0[j]: st[j + 1] - st[j] for j in range(len(names0))}
print("消えた列の元の非零の数の分布:", sorted(collections.Counter(min(deg[x], 10) for x in gone).items()))
print("残った列の元の非零の数の分布:", sorted(collections.Counter(min(deg[x], 10) for x in sa).items()))
# 集約 (規則 12) で消えた列 (プロービング・列挙なし)
_, c = presolved((1 << 15) | (1 << 16))
sc = set(c.col_names_)
g2 = sb - sc
print("\n集約あり・プロービングなし:", len(sc), "列", collections.Counter(kind[x] for x in sc), "行", c.num_row_, "(集約なし", b.num_row_, "行)")
print("集約で消えた列:", len(g2), collections.Counter(kind[x] for x in g2), "非零の数の分布:", sorted(collections.Counter(min(deg[x], 10) for x in g2).items()))
_, d = presolved(1 << 16)
sd = set(d.col_names_)
print("集約 + プロービング (列挙なし):", len(sd), "列", collections.Counter(kind[x] for x in sd), "行", d.num_row_)

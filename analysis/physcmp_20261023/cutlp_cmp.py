"""ENOMOTO のカット入りの LP (ENOMOTO_MIP_DEBUG_DUMP_CUT_LP の cutlp_r*.mps、列 c<j> = 前処理後の列 j) を調べる。
python cutlp_cmp.py <cutlp mps> <基の行数> [<HiGHS のプール json (列番号は ENOMOTO の dump と同じ)>]
- 基の行 (前処理後 + クリーク拡張 + 対称性の行) の後ろをカットとみなして形で分類し、双対非零の本数を数える
- HiGHS のプールを足した LP の値、HiGHS の双対非零のカットのうち ENOMOTO の LP 解が破るもの"""
import json, sys
import numpy as np, highspy
from collections import Counter
mps, nbase = sys.argv[1], int(sys.argv[2])
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(mps); lp = h.getLp()
n, m = lp.num_col_, lp.num_row_
lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
integ = list(lp.integrality_); isint = [integ[j] != highspy.HighsVarType.kContinuous for j in range(n)] if integ else [True] * n
h.setOptionValue("presolve", "on"); h.run()
base = h.getInfo().objective_function_value
sol = h.getSolution(); x = np.array(sol.col_value); dual = np.array(sol.row_dual)
A = lp.a_matrix_; st, ix, vl = np.array(A.start_), np.array(A.index_), np.array(A.value_)
rows = [[] for _ in range(m)]
for j in range(n):
    for p in range(st[j], st[j + 1]): rows[ix[p]].append((j, vl[p]))
rl, ru = np.array(lp.row_lower_), np.array(lp.row_upper_)
binary = [isint[j] and lo[j] == 0 and up[j] == 1 for j in range(n)]
def kind(r, ub, lb):
    idx = [j for j, _ in r]; val = [v for _, v in r]
    k = len(idx); allbin = all(binary[j] for j in idx); unit = all(abs(abs(v) - 1) < 1e-9 for v in val)
    if allbin and unit and k == 2: return "2変数 ±1 (含意)"
    if allbin and unit:
        nneg = sum(1 for v in val if v < 0)
        if (ub < 1e30 and abs(ub - (1 - nneg)) < 1e-9) or (lb > -1e30 and abs(-lb - (1 - (k - nneg))) < 1e-9): return "クリーク (文字の和<=1)"
        return "0-1 ±1 (その他)"
    if all(isint[j] for j in idx): return "整数のみ (一般係数)"
    return "連続列を含む (MIR 型)"
kinds = [kind(rows[i], ru[i], rl[i]) for i in range(nbase, m)]
cd = dual[nbase:]
print(f"{mps}: LP {base:.4f}, cut rows {m - nbase}")
cnt = Counter(kinds); act = Counter(k for k, d in zip(kinds, cd) if abs(d) > 1e-7)
for k in cnt: print(f"  {k}: {cnt[k]} 本、双対非零 {act[k]}")
if len(sys.argv) > 3:
    P = json.load(open(sys.argv[3])); S, I, V = P["start"], P["index"], P["value"]
    actset = set(json.load(open(sys.argv[3] + ".active.json"))["active"])
    PK = []
    viol = Counter(); vk = []
    for c in range(len(S) - 1):
        idx = I[S[c]:S[c + 1]]; val = V[S[c]:S[c + 1]]
        a = float(np.dot(x[idx], val))
        v = max(P["lower"][c] - a, a - P["upper"][c])
        k = kind(list(zip(idx, val)), P["upper"][c], P["lower"][c])
        if v > 1e-6:
            viol[k] += 1
            if c in actset: vk.append((c, k, v))
        h.addRow(P["lower"][c], P["upper"][c], len(idx), np.array(idx, dtype=np.int32), np.array(val))
    print("  HiGHS のプールのうち ENOMOTO の LP 解が破るもの:", dict(viol))
    print("  そのうち HiGHS で双対非零のもの:", len(vk), Counter(k for _, k, _ in vk))
    for c, k, v in vk: print(f"    cut {c} [{k}] viol {v:.3f}")
    h.run(); print(f"  + HiGHS のプール: {h.getInfo().objective_function_value:.4f}")

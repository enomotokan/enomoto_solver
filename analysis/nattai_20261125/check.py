"""x_*.json の解を元の MPS (前処理の前) に代入し、行・境界・整数性の違反の最大値と目的値を計算し直す。
MPS は highspy で読む (このクレートの読み込みとは別の実装で確かめるため)。使い方: python check.py x_combo_t1.json ..."""
import json, sys
import numpy as np, highspy
from pathlib import Path
R = Path(__file__).resolve().parents[2]
h = highspy.Highs(); h.setOptionValue("output_flag", False)
h.readModel(str(R / ".miplib_cache/neos-5188808-nattai.mps"))
lp = h.getLp(); n, mrow = lp.num_col_, lp.num_row_
names = list(lp.col_names_)
A = lp.a_matrix_; start, idx, val = np.array(A.start_), np.array(A.index_), np.array(A.value_)
integ = np.array([int(t) for t in lp.integrality_]) if len(lp.integrality_) else np.zeros(n, int)
cl, cu, rl, ru, c = map(np.array, (lp.col_lower_, lp.col_upper_, lp.row_lower_, lp.row_upper_, lp.col_cost_))
for f in sys.argv[1:]:
    d = json.loads(Path(f).read_text()); xd = d["x"]
    x = np.array([xd[nm] for nm in names])
    act = np.zeros(mrow)
    for j in range(n):
        s, e = start[j], start[j + 1]; act[idx[s:e]] += val[s:e] * x[j]
    rv = np.maximum(rl - act, act - ru); bv = np.maximum(cl - x, x - cu)
    iv = np.where(integ == 1, np.abs(x - np.round(x)), 0.0)
    obj = float(c @ x) + lp.offset_
    i = int(np.argmax(rv)); k = int(np.argmax(bv)); q = int(np.argmax(iv))
    # 相対の行違反 (行の係数と x の大きさで割る)
    print(f"{f}: 報告の目的値 {d['objective']} / 計算し直し {obj:.10g}")
    print(f"  行違反の最大 {rv[i]:.3e} (行 {i} {lp.row_names_[i] if len(lp.row_names_) else ''}, 活動 {act[i]:.10g}, 範囲 [{rl[i]:.6g}, {ru[i]:.6g}])")
    print(f"  境界違反の最大 {bv[k]:.3e} (列 {names[k]})、整数性違反の最大 {iv[q]:.3e} (列 {names[q]} = {x[q]!r})")
    print(f"  1e-6 を超える行違反 {int((rv > 1e-6).sum())} 行、1e-9 超 {int((rv > 1e-9).sum())} 行")

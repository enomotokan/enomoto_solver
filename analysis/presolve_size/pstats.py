"""前処理後の問題 (MPS) の形と LP 緩和の値を出す。使い方: python3 pstats.py <mps> [目的の定数]
出力 (JSON 1 行): rows, cols (固定でない列), nnz (固定でない列の非零), bin, int, cont, eq, lp (LP 緩和の値 + 定数)"""
import sys, json, highspy, numpy as np
h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("threads", 1)
h.readModel(sys.argv[1]); off = float(sys.argv[2]) if len(sys.argv) > 2 else 0.0
lp = h.getLp()
lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
st = np.array(lp.a_matrix_.start_)
integ = np.array([int(v) for v in lp.integrality_]) if len(lp.integrality_) else np.zeros(lp.num_col_, int)
free = lo < up
isint = integ != 0
isbin = isint & (lo == 0) & (up == 1)
rl, ru = np.array(lp.row_lower_), np.array(lp.row_upper_)
nnz = int(sum(st[j + 1] - st[j] for j in np.nonzero(free)[0]))
d = dict(rows=lp.num_row_, cols=int(free.sum()), nnz=nnz, bin=int((free & isbin).sum()), int=int((free & isint & ~isbin).sum()),
         cont=int((free & ~isint).sum()), eq=int((rl == ru).sum()))
h.setOptionValue("solve_relaxation", True); h.setOptionValue("presolve", "off"); h.setOptionValue("time_limit", 120.0)
h.run()
d["lp"] = h.getInfo().objective_function_value + off if h.getModelStatus() == highspy.HighsModelStatus.kOptimal else None
print(json.dumps(d))

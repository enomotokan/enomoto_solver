"""binkar10_1: 返した解 (整数列) を固定し、行・境界をこのソルバーの許容誤差 (1e-6 + 1e-9|b|) だけ緩めた LP の最小値を HiGHS で求める (2026-11-05)。
許容誤差の中で目的値がどこまで下がるかを見る。使い方: python tolslack.py <x.npy> [緩める幅の係数 既定 1]"""
import sys, numpy as np, highspy
from pathlib import Path
x = np.load(sys.argv[1]); k = float(sys.argv[2]) if len(sys.argv) > 2 else 1.0
h = highspy.Highs(); h.silent(); h.readModel(str(Path(__file__).resolve().parents[2] / ".miplib_cache/binkar10_1.mps"))
lp = h.getLp(); n, m = lp.num_col_, lp.num_row_
integ = np.array([int(v) for v in lp.integrality_])
lo = np.array(lp.col_lower_); up = np.array(lp.col_upper_); rl = np.array(lp.row_lower_); ru = np.array(lp.row_upper_)
tol = lambda b: k * (1e-6 + 1e-9 * np.abs(np.where(np.isfinite(b), b, 0)))
for j in range(n):
    if integ[j]:
        h.changeColBounds(j, round(x[j]), round(x[j]))
    else:
        h.changeColBounds(j, lo[j] - tol(lo[j]) if np.isfinite(lo[j]) else lo[j], up[j] + tol(up[j]) if np.isfinite(up[j]) else up[j])
for i in range(m):
    h.changeRowBounds(i, rl[i] - tol(rl[i]) if np.isfinite(rl[i]) else rl[i], ru[i] + tol(ru[i]) if np.isfinite(ru[i]) else ru[i])
h.changeColsIntegrality(n, np.arange(n, dtype=np.int32), np.zeros(n, dtype=np.uint8))
h.run(); print("relaxed by", k, "x tol: LP min", h.getInfo().objective_function_value, h.modelStatusToString(h.getModelStatus()))

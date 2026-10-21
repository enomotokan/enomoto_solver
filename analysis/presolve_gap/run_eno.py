"""ENOMOTO で 1 問を解く (4 スレッド・既定 60 秒)。使い方: python run_eno.py <mps> [秒]。環境変数はそのまま渡る。
返った解を元の問題 (HiGHS で読んだもの) で確かめ、最大の行の違反・境界の違反・整数性の違反・目的値を CHECK 行に出す。"""
import sys, time
import numpy as np
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
mps = sys.argv[1]
T = float(sys.argv[2]) if len(sys.argv) > 2 else 60.0
_, lp = bh._load_lp(Path(mps))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter()
r = m.solve(None, False, T, None, None, 4, log=True)
print("RESULT", r["status"], r["objective"], r["best_bound"], r["nodes"], round(time.perf_counter() - t, 1), r.get("root_bound"), flush=True)
x = r.get("x")
if x is not None and len(x) == lp.num_col_:
    x = np.array(x, float)
    st = np.array(lp.a_matrix_.start_); idx = np.array(lp.a_matrix_.index_); val = np.array(lp.a_matrix_.value_)
    act = np.zeros(lp.num_row_)
    for j in range(lp.num_col_):
        act[idx[st[j]:st[j + 1]]] += val[st[j]:st[j + 1]] * x[j]
    rl, ru = np.array(lp.row_lower_), np.array(lp.row_upper_)
    rviol = np.maximum(np.maximum(rl - act, act - ru), 0.0).max(initial=0.0)
    lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
    bviol = np.maximum(np.maximum(lo - x, x - up), 0.0).max(initial=0.0)
    integ = np.array([int(v) for v in lp.integrality_]) if len(lp.integrality_) else np.zeros(lp.num_col_, int)
    iviol = np.abs(x - np.round(x))[integ != 0].max(initial=0.0)
    obj = float(np.dot(np.array(lp.col_cost_), x) + lp.offset_)
    ok = rviol <= 1e-6 and bviol <= 1e-6 and iviol <= 1e-6 and (r["objective"] is None or abs(obj - r["objective"]) <= 1e-6 * max(1, abs(obj)))
    print(f"CHECK {'OK' if ok else 'BAD'} rowviol {rviol:.2e} bndviol {bviol:.2e} intviol {iviol:.2e} obj {obj}", flush=True)
else:
    print("CHECK nosol", flush=True)

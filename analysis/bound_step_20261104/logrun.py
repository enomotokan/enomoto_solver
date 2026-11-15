"""1 問を Model.solve(log=True) で解き、利用者向けのログ (経過の行) を標準出力に出す (2026-11-04、bound_step の確かめ)。
使い方: python analysis/bound_step_20261104/logrun.py <MPS> <秒> [スレッド数]   (ENOMOTO_MIP_BOUND_STEP=1 などは環境変数で)"""
import sys
from pathlib import Path
from enomoto_solver import benchmark_highs as bh

_, lp = bh._load_lp(Path(sys.argv[1]))
m, _, _ = bh._build_our_model(lp)
th = int(sys.argv[3]) if len(sys.argv) > 3 else 4
r = m.solve(time_limit=float(sys.argv[2]), threads=th, log=True)
print("RESULT", r.status, r.objective_value if hasattr(r, "objective_value") else "")

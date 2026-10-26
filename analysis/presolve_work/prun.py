"""ENOMOTO で 1 問を解き始め、前処理 (クリーク行の拡張・対称性の行の後) で止める。使い方: python3 prun.py <mps> <秒> <スレッド>
`ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3` と `ENOMOTO_MIP_DEBUG_STOP_PRESOLVED` は呼び出し側 (pone.sh) が付ける。"""
import sys
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path(sys.argv[1]))
m, _, _ = bh._build_our_model(lp)
m.solve(None, False, float(sys.argv[2]), None, None, int(sys.argv[3]), log=True)
print("NOT STOPPED", flush=True)

"""ENOMOTO で 1 問を解き始める (前処理後の問題を書き出すため。呼び出し側 (eone.sh) が書き出しを見て止める)。
時間制限は 120 秒 (前処理の時間の上限・打ち切りに掛からないよう長くとる)。使い方: python3 run1.py <mps>"""
import sys
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path(sys.argv[1]))
m, _, _ = bh._build_our_model(lp)
m.solve(None, False, 120.0, None, None, 1, log=True)

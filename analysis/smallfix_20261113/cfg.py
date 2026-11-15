"""SMALLFIX (2026-11-13) の設定。土台は cand1114 (analysis/cand1114_20261113/cfg.py の CAND1114)。
c3: clique fallback の試しを、ループで目的値が動いていて破れる clique カットの効き目の最大が 0.2 未満なら省き (`s`)、LP に時間の上限 max(4 秒, 3 x ループの 1 ラウンドの平均の LP の秒) をつける (`t`)。
d1: 新たな固定 0 列の木の中の再スタートを止める。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1114", Path(__file__).resolve().parents[1] / "cand1114_20261113" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
CAND1114 = _m.CAND1114
C3 = {"ENOMOTO_MIP_CLQFB_CAP": "st"}
D1 = {"ENOMOTO_MIP_TRNOFIX": "1"}
D1A1 = {"ENOMOTO_MIP_TRNOFIX": "1", "ENOMOTO_T_MIP_TRNOFIX_ALLOW": "1"}  # d1 の変形: 固定 0 列の再スタートを 1 回までは通す
D1G = {"ENOMOTO_MIP_TRNOFIX": "g"}  # d1 の変形: 固定 0 列の再スタートは 1 回目は通し、その後の根の下界の伸びがギャップの 0.05 倍未満なら次から止める
CFGS = {"base": dict(CAND1114), "c3": {**CAND1114, **C3}, "d1": {**CAND1114, **D1}, "c3d1": {**CAND1114, **C3, **D1}, "d1a1": {**CAND1114, **D1A1}, "d1g": {**CAND1114, **D1G}}

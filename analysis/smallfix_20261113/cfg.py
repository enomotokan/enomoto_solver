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
CFGS = {"base": dict(CAND1114), "c3": {**CAND1114, **C3}, "d1": {**CAND1114, **D1}, "c3d1": {**CAND1114, **C3, **D1}}

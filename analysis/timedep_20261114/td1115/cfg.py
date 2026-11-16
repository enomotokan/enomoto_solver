"""cand1115 の TIMEDEP Et2 の 1 文字ずつ外し (analysis/timedep_20261114.md の 9 節)。束は analysis/cand1115_20261115/cfg.py の CAND1115 (TC=b、Et2 を含む)。
cand1115 の 1 つ外し (loo_summ.txt・loom_summ.txt) で TIMEDEP を外すと neos17・neos-911970・physiciansched6-2 が速くなり、tbfp-network (600 秒) が解けた。
どの文字・調整値が原因かを見る。設定名: full = CAND1115、-all = TIMEDEP を全部外す (CAND1115C)、-x = 文字 x を外す、-sc・-lns = 調整値を既定に戻す。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1115", Path(__file__).resolve().parents[2] / "cand1115_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
CAND1115, CAND1115C = _m.CAND1115, _m.CAND1115C
L = CAND1115["ENOMOTO_MIP_TIMEDEP"]
assert L == "mplwnNebytu"
def drop(x):
    v = L.replace(x, "")
    if x == "n": v = v.replace("N", "")
    return {**CAND1115, "ENOMOTO_MIP_TIMEDEP": v}
CFGS = {"full": dict(CAND1115), "-all": dict(CAND1115C)}
for x in "mplwnNebytu":
    CFGS[f"-{x}"] = drop(x)
CFGS["-sc"] = {k: v for k, v in CAND1115.items() if k != "ENOMOTO_T_MIP_TIMEDEP_SEP_COST"}
CFGS["-lns"] = {k: v for k, v in CAND1115.items() if k != "ENOMOTO_T_MIP_TIMEDEP_LNS_MIN"}
P60 = ["neos17", "neos-911970", "physiciansched6-2"]
P600 = ["tbfp-network"]

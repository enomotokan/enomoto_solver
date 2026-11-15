"""densecut の設定。土台は束 cand1114 (analysis/cand1114_20261113/cfg.py)。"""
import importlib.util
import sys
from pathlib import Path
D = Path(__file__).resolve().parent
_sp = importlib.util.spec_from_file_location("cfg1114", D.parent / "cand1114_20261113" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
CAND1114 = _m.CAND1114
sys.path.insert(0, str(D))
import plan_m  # noqa: E402
CFGS = {
    "c": dict(CAND1114),
    "c0": dict(CAND1114, ENOMOTO_T_MIP_TREE_SEP_ROUNDS="0"),   # ノードの分離を止める (木の分離のラウンドを 0)
    "ci": dict(CAND1114, ENOMOTO_MIP_RCSTRONG="irk"),            # rmatr100-p10 の i 形 (根の大域的な固定で連続列が締まる)
    "ckR": dict(CAND1114, ENOMOTO_MIP_RCSTRONG_KROOT="1"),       # k + KROOT (rcstrong4)
    "ci0": dict(CAND1114, ENOMOTO_MIP_RCSTRONG="irk", ENOMOTO_T_MIP_TREE_SEP_ROUNDS="0"),
    "ckR0": dict(CAND1114, ENOMOTO_MIP_RCSTRONG_KROOT="1", ENOMOTO_T_MIP_TREE_SEP_ROUNDS="0"),
    # 試作 ENOMOTO_MIP_DENSECUT (src/mip/solver.rs の densecut_has)
    "ca": dict(CAND1114, ENOMOTO_MIP_DENSECUT="a"),
    "cg": dict(CAND1114, ENOMOTO_MIP_DENSECUT="g"),
    "cag": dict(CAND1114, ENOMOTO_MIP_DENSECUT="ag"),
}
MSET = plan_m.M

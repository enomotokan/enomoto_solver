"""REVIEW3 (2026-11-13) の設定。土台 base = 束 cand1114 (analysis/cand1114_20261113/cfg.py)。REVIEW2 (analysis/review2_20261113.md の (D)) の
候補を 1 つずつ足す: 根の 1 ラウンドのカットの本数の上限 (`ENOMOTO_T_MIP_ROOT_MAX_CUTS`、既定 50) を 100・150・300 にした形と、MUSHAGG を外した形。"""
import importlib.util
from pathlib import Path
_spec = importlib.util.spec_from_file_location("cand1114_cfg", Path(__file__).resolve().parents[1] / "cand1114_20261113" / "cfg.py")
_m = importlib.util.module_from_spec(_spec); _spec.loader.exec_module(_m)
BASE = dict(_m.CAND1114)
ITEMS = {
    "rmc100": {"ENOMOTO_T_MIP_ROOT_MAX_CUTS": "100"},
    "rmc150": {"ENOMOTO_T_MIP_ROOT_MAX_CUTS": "150"},
    "rmc300": {"ENOMOTO_T_MIP_ROOT_MAX_CUTS": "300"},
    "mush0": {"ENOMOTO_MIP_MUSHAGG": "0"},
}
CFGS = {"base": BASE, **{k: {**BASE, **v} for k, v in ITEMS.items()}}

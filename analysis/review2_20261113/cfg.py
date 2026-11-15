"""REVIEW2 (2026-11-13) の設定。土台 base = 束 cand1113 (analysis/cand1113_20261112/cfg.py) の RCSTRONG を `k` にした形。
ほかの設定は base に 1 項目だけ足した (変えた) 形。項目の理由は analysis/review2_20261113.md の (C)。"""
import importlib.util
from pathlib import Path
_spec = importlib.util.spec_from_file_location("cand1113_cfg", Path(__file__).resolve().parents[1] / "cand1113_20261112" / "cfg.py")
_m = importlib.util.module_from_spec(_spec); _spec.loader.exec_module(_m)
BASE = dict(_m.CAND1113)
BASE["ENOMOTO_MIP_RCSTRONG"] = "k"

ITEMS = {
    # (B) 数値
    "hst15": {"ENOMOTO_T_MIP_SUBMIP_HIGHS_TIME_FRAC": "0.15"},          # 根のサブ MIP の時間: 残りの全部 → 残りの 15%
    "cr50": {"ENOMOTO_T_MIP_CUT_ROUNDS": "50",                            # 根のカットのラウンドの上限 25 → 50 (外側の根だけ)
             "ENOMOTO_T_MIP_SUBMIP_HC_CUT_ROUNDS": "25"},
    "rmc150": {"ENOMOTO_T_MIP_ROOT_MAX_CUTS": "150"},                     # 根の 1 ラウンドのカットの本数の上限 50 → 150
    "prb3": {"ENOMOTO_T_MIP_PRESOLVE_WORK_PROBE0": "3.45e9",              # プロービングの予算を 3 倍 (手間・時間とも)
             "ENOMOTO_T_MIP_PRESOLVE_WORK_PROBE": "9e9",
             "ENOMOTO_T_MIP_PROBE_ADAPT_WORK": "6000",
             "ENOMOTO_T_MIP_PROBE_ADAPT_TIME0": "3",
             "ENOMOTO_T_MIP_CLQPRB_TIME": "18"},
    "prr": {"ENOMOTO_T_MIP_PROBE_REPRESOLVE_MAX": "4",                    # プロービング後のかけ直し 2 → 4 回、捨てる閾値 0.1 → 0.03
            "ENOMOTO_T_MIP_PRR_MIN_ROW_FRAC": "0.03"},
    "lns20": {"ENOMOTO_T_MIP_SUBMIP_TIME_MAX": "20"},                     # 木の大近傍探索のサブ MIP の時間の上限 6 → 20 秒
    # (A) 1 つずつ外して確かめていない部品
    "nlp0": {"ENOMOTO_MIP_NODELP_PERTURB": "0"},
    "csp0": {"ENOMOTO_MIP_CUTSEL_PROBE": "0"},
    "bgd0": {"ENOMOTO_MIP_BG_CUTS_DETERM": "0"},
    "mush0": {"ENOMOTO_MIP_MUSHAGG": "0"},
}
CFGS = {"base": BASE}
for k, v in ITEMS.items():
    CFGS[k] = {**BASE, **v}

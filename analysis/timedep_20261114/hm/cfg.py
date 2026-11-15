"""SUBMIP_HALT の Mittelmann 版 (analysis/timedep_20261114.md の 9 節、4 スレッド・600 秒、種 1・2)。問題は cand1115 の loom (loom_run.py) の 8 問。
base = CAND1115C、cH = base + SUBMIP_HALT、tdH = CAND1115 (TIMEDEP Et2 入り) + SUBMIP_HALT。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1115", Path(__file__).resolve().parents[2] / "cand1115_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
H = {"ENOMOTO_MIP_SUBMIP_HALT": "1"}
CFGS = {"base": dict(_m.CAND1115C), "cH": {**_m.CAND1115C, **H}, "tdH": {**_m.CAND1115, **H}}
PROBS = ["ran14x18-disj-8", "snp-02-004-104", "fiball", "tbfp-network", "CMS750_4", "fast0507", "rocI-4-11", "traininstance6"]

"""手間の単位 (`ENOMOTO_MIP_TICKS`) の Mittelmann 版 (4 スレッド・600 秒、種 1・2)。問題は cand1115 の loom の 8 問。
base = cand1116f、K = base + `ENOMOTO_MIP_TICKS=ntwu`。"""
import importlib.util
from pathlib import Path
_sp = importlib.util.spec_from_file_location("cfg1116", Path(__file__).resolve().parents[2] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
CFGS = {"base": dict(_m.CAND1116F), "K": {**_m.CAND1116F, "ENOMOTO_MIP_TICKS": "ntwu"}}
PROBS = ["ran14x18-disj-8", "snp-02-004-104", "fiball", "tbfp-network", "CMS750_4", "fast0507", "rocI-4-11", "traininstance6"]

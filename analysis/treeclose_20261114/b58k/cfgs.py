"""b58k の設定: 束 cand1116 の上に、ベイズ最適化の提案 (proposals_r*.json の env) を重ねたもの。"""
import importlib.util, json
from pathlib import Path
D = Path(__file__).resolve().parent
_sp = importlib.util.spec_from_file_location("cfg1116", D.parents[1] / "cand1116_20261115" / "cfg.py")
_m = importlib.util.module_from_spec(_sp); _sp.loader.exec_module(_m)
BASE = dict(_m.CAND1116)
CFGS = {"base": BASE}
for f in sorted(D.glob("proposals_r*.json")):
    for c, v in json.loads(f.read_text())["cfgs"].items():
        CFGS[c] = dict(BASE, **v["env"])

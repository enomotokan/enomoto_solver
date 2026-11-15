"""調査 2 の問題の一覧: 58 問 (cbcimpl_20261029/probs.py) と Mittelmann (mitt_parts.json) の和 (physiciansched6-2 を除く)。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
MITT = [p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part]
ALL = [p for p in dict.fromkeys(PROBS + MITT) if p != "physiciansched6-2"]
if __name__ == "__main__":
    print(len(PROBS), len(MITT), len(ALL))

"""この記録で使う問題: 58 問 (analysis/cbcimpl_20261029/probs.py) と HiGHS に負ける 47 問 (analysis/mitt600_20261103/shards8.json)。
python probs_all.py で MPS を取ってくる (.miplib_cache)。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
sys.path.insert(0, str(WT / "scripts"))
from probs import PROBS as P58  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402
P47 = json.load(open(WT / "analysis" / "mitt600_20261103" / "shards8.json"))["problems"]
ALL = P58 + [p for p in P47 if p not in P58]
if __name__ == "__main__":
    for p in ALL:
        try:
            print(p, rb.fetch(p, WT / ".miplib_cache"), flush=True)
        except Exception as e:  # noqa: BLE001
            print(p, "FAIL", e, flush=True)

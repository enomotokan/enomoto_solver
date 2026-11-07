"""数える問題: 58 問 (analysis/cbcimpl_20261029/probs.py) と Mittelmann 186 問 (analysis/mitt600_20261103/shards2.json)。
python gprobs.py で全部を .miplib_cache に取得する。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
sys.path.insert(0, str(WT / "scripts"))
import probs as p58
import run_miplib_benchmark as rb
MITT = list(dict.fromkeys(p for s in json.load(open(WT / "analysis/mitt600_20261103/shards2.json"))["shards"] for p in s))
SETS = {"p58": p58.PROBS, "mitt": MITT}
if __name__ == "__main__":
    for p in dict.fromkeys(p58.PROBS + MITT):
        try:
            print(p, rb.fetch(p, WT / ".miplib_cache"), flush=True)
        except Exception as e:
            print(p, "FAIL", e, flush=True)

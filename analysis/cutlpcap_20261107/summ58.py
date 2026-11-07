"""58 問 (4 スレッド・60 秒、種 1・2) の def 対 ac の集計 (analysis/benchlib.py の report)。使い方: python summ58.py > summ58.txt"""
import sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent))
import benchlib as bl  # noqa: E402
R = bl.table(bl.load([D / "bench58.jsonl"]))
keys = sorted(k for k in R if "def" in R[k] and "ac" in R[k])
bl.report(R, "def", "ac", keys, noise=bl.load_noise(), label="58 問")
print()
print("1.5 倍以上違った回 (時間、未解決は 60):")
for k in keys:
    a, b = bl.t(R[k]["def"]), bl.t(R[k]["ac"])
    if max(a, b) + 10 > 1.5 * (min(a, b) + 10):
        print(f"  {k[0]:28s} r{k[1]} def {a:6.1f} ac {b:6.1f}")

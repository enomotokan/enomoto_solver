"""47 問の対称性のある 13 問 (600 秒) の集計: python summ47.py"""
import sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import benchlib as bl  # noqa: E402
bl.TL = 600.0
for f in (bl.ok, bl.t, bl.lt):
    f.__defaults__ = (600.0,)
R = bl.table(bl.load([HERE / "res47g.jsonl"]))
for A, B in (("def", "dynf"),):
    keys = sorted(k for k in R if A in R[k] and B in R[k])
    bl.report(R, A, B, keys, label=f"{A} vs {B} (600 秒、{len(keys)} 回)")
    print()

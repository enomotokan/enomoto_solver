"""60 秒の比べの集計 (benchlib.report): python summ58.py [res58.jsonl]"""
import sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import benchlib as bl  # noqa: E402
rows = bl.load([HERE / (sys.argv[1] if len(sys.argv) > 1 else "res58.jsonl")])
R = bl.table(rows)
keys = sorted(R)
noise = bl.load_noise()
for B in ["b", "a"]:
    bl.report(R, "def", B, keys, noise=noise, label=f"def vs {B}")
    print()

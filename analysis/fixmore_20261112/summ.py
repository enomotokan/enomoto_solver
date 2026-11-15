"""FIXMORE の 58 問の集計 (analysis/fixmore_20261112.md): python summ.py [設定,...]
res_s*.jsonl を読み、既定 (def) と各設定を benchlib.report で比べる。"""
import sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import benchlib as bl  # noqa: E402
rows = bl.load(sorted([f for f in HERE.glob("res*_s*.jsonl") if ".live" not in f.name]))
R = bl.table(rows)
noise = bl.load_noise()
for B in (sys.argv[1] if len(sys.argv) > 1 else "u").split(","):
    keys = sorted(k for k in R if "def" in R[k] and B in R[k])
    bl.report(R, "def", B, keys, noise=noise, label=f"def vs {B} ({len(keys)} 回)")
    print()

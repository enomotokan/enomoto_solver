"""58 問 (60 秒、種 1・2) の既定と ple の比べ (benchlib の report)。使い方: python summ58.py"""
import sys
from pathlib import Path
here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parent))
import benchlib as bl  # noqa: E402
R = bl.table(bl.load([here / "b58.jsonl"]))
keys = [k for k in R if "def" in R[k] and "ple" in R[k]]
print(f"対の数 {len(keys)}")
bl.report(R, "def", "ple", keys, noise=bl.load_noise())

"""COMP07: res58.jsonl の 2 つの設定を benchlib で比べる (SGM・対の比の 95% 区間・解けた数・揃って遅い問題)。
使い方: python analysis/comp07_20261111/cmp58.py [A B]   (既定 def fjkeep)"""
import sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent))
import benchlib as bl  # noqa: E402
A, B = (sys.argv[1:3] if len(sys.argv) > 2 else ("def", "fjkeep"))
R = bl.table(bl.load([D / "res58.jsonl"]))
keys = sorted(k for k in R if A in R[k] and B in R[k])
bl.report(R, A, B, keys, noise=bl.load_noise(), label=f"{A} vs {B} ({len(keys)} 回)")

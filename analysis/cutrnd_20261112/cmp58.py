"""CUTRND: res58*.jsonl の 2 つの設定を benchlib で比べる (SGM・対の比の 95% 区間・解けた数・揃って遅い問題)。
使い方: python analysis/cutrnd_20261112/cmp58.py [A B [ファイルの glob]]   (既定 def cr、"res58_sh*.jsonl"。分担の結果は "res58_sh*.jsonl")"""
import sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent))
import benchlib as bl  # noqa: E402
A, B = (sys.argv[1:3] if len(sys.argv) > 2 else ("def", "cr"))
files = sorted(D.glob(sys.argv[3])) if len(sys.argv) > 3 else sorted(D.glob("res58*.jsonl"))
R = bl.table(bl.load(files))
keys = sorted(k for k in R if A in R[k] and B in R[k])
bl.report(R, A, B, keys, noise=bl.load_noise(), label=f"{A} vs {B} ({len(keys)} 回)")
print("consistent:", bl.consistent(bl.paired(R, A, B, keys)))

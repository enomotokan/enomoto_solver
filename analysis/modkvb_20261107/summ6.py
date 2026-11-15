"""6 章: 58 問の 60 秒 (b58.jsonl、種 1・2) で既定・zerohalf なし (nozh6)・mk50・mk50 + zerohalf なし (mk50nozh6)・zerohalf と平行な候補を除く
mk50 (mk50skz) を benchlib の report で比べる。そろった (問題, 回) だけ。使い方: python summ6.py"""
import sys
from pathlib import Path
here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parent))
import benchlib as bl  # noqa: E402
R = bl.table(bl.load([here / "b58.jsonl"]))
noise = bl.load_noise()
for a, b in [("def", "nozh6"), ("def", "mk50"), ("def", "mk50nozh6"), ("def", "mk50skz"), ("mk50", "mk50skz"), ("nozh6", "mk50nozh6")]:
    ks = [k for k in R if a in R[k] and b in R[k]]
    bl.report(R, a, b, ks, noise=noise, label=f"58 問 {a}→{b}", show_root=False)
    print()

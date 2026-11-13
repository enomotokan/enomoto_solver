"""iad (cand1114 + ENOMOTO_MIP_TREE2_INTAGG=d、analysis/presolvegap_20261113.md) の 58 問 x 種 1〜3 の対の計測 (iad58_s*.jsonl) を analysis/benchlib.py の report で集計する。
誤答 (目的値・下界の矛盾) も見る。
使い方: python summ_tubr58.py"""
import glob, json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent))
import benchlib as bl  # noqa: E402
rows = bl.load(sorted(glob.glob(str(D / "iad58_s*.jsonl"))))
R = bl.table(rows)
keys = sorted(R)
bl.report(R, "base", "iad", keys, noise=bl.load_noise(), label="58 問 x 種 1〜3")
# 誤答: 最適と言った回どうしの目的値の差、下界が他の回の最適値を越えるもの (最小化)
bad = 0
for p in {k[0] for k in keys}:
    res = [R[k][c] for k in keys if k[0] == p for c in R[k]]
    opt = [x["objective"] for x in res if (x.get("status") or "").lower() == "optimal" and x.get("objective") is not None]
    if not opt:
        continue
    lo, hi = min(opt), max(opt)
    tol = 1e-6 * max(1.0, abs(lo))
    if hi - lo > tol:
        bad += 1; print("目的値の不一致", p, lo, hi)
    for x in res:
        b = x.get("bound")
        if b is not None and b > lo + tol:
            bad += 1; print("下界が最適値を越える", p, b, lo)
print("誤答", bad)

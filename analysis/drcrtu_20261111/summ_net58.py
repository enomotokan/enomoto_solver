"""net (ENOMOTO_MIP_FOLD=rinq) の 58 問 x 種 1〜3 の対の計測 (net58_s*.jsonl) を analysis/benchlib.py の report で集計する。
u が働いた問題 (detect_u.py の u_hits.jsonl で、LP の試しで列を固定したか最適を示したもの) は別に出す。誤答 (目的値・下界の矛盾) も見る。
使い方: python summ_net58.py"""
import glob, json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent))
import benchlib as bl  # noqa: E402
rows = bl.load(sorted(glob.glob(str(D / "net58_s*.jsonl"))))
R = bl.table(rows)
keys = sorted(R)
bl.report(R, "def", "net", keys, noise=bl.load_noise(), label="58 問 x 種 1〜3")
hk = [k for k in keys if k[0] == "physiciansched6-2"]
bl.report(R, "def", "net", hk, label="畳み方が変わる physiciansched6-2", show_root=False)
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

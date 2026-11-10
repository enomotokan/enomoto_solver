"""(b) 58 問の集計 (benchlib.report) と、暫定解・下界の矛盾の検査、最初の解の時刻の比べ。"""
import json, math, sys
from pathlib import Path
here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parent))
import benchlib as bl  # noqa: E402
f = sys.argv[1] if len(sys.argv) > 1 else "b58.jsonl"
R = bl.table(bl.load([here / f]))
keys = sorted(k for k in R if "def" in R[k] and "fpr" in R[k])
try:
    noise = bl.load_noise()
except Exception:  # noqa: BLE001
    noise = None
bl.report(R, "def", "fpr", keys, noise=noise, label=f)
# 暫定解と下界の矛盾 (問題ごとに全回の下界と目的値が同じ側にあるか)
by = {}
for k in R:
    for c, r in R[k].items():
        by.setdefault(k[0], []).append(r)
bad = []
for p, rs in by.items():
    obj = [r["objective"] for r in rs if r.get("objective") is not None and math.isfinite(r["objective"])]
    bd = [r["bound"] for r in rs if r.get("bound") is not None and math.isfinite(r["bound"])]
    if not obj or not bd:
        continue
    tol = 1e-6 * max(1.0, max(abs(v) for v in obj))
    ok_min = max(bd) <= min(obj) + tol
    ok_max = min(bd) >= max(obj) - tol
    if not (ok_min or ok_max):
        bad.append((p, min(obj), max(obj), min(bd), max(bd)))
print(f"暫定解と下界の矛盾: {len(bad)} 問", bad)
# 最初の解の時刻
print("最初の解の時刻 (既定 → FPR、どちらかが 1 秒以上ちがう回):")
for k in keys:
    a, b = R[k]["def"], R[k]["fpr"]
    ta, tb = a.get("first_solution_time"), b.get("first_solution_time")
    if ta is None or tb is None or abs(ta - tb) >= 1.0:
        print(f"  {k[0]:22s} s{k[1]} {ta} → {tb}  値 {a.get('first_solution_objective')} → {b.get('first_solution_objective')}  時間 {a.get('time'):.1f} → {b.get('time'):.1f} {a.get('status')}/{b.get('status')}")

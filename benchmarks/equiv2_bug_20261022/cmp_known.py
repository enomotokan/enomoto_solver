"""check55.jsonl の下界・最適値を、過去の計測 (benchmarks/miplib_results*.json・highsonly16*.json) の HiGHS の解の値と比べる。
最小化に直した値で、下界 > 既知の解 + 許容 (相対 1e-6) なら誤り、最適と言った値 > 既知の解 + 相対 1.5e-4 (既定のギャップ) なら要確認。"""
import json, sys, glob
from pathlib import Path
root = Path(__file__).resolve().parents[2]
best = {}
for f in glob.glob(str(root / "benchmarks/miplib_results*.json")) + glob.glob(str(root / "benchmarks/highsonly16*.json")):
    try: d = json.load(open(f))
    except Exception: continue
    if not isinstance(d, dict): continue
    for p, v in d.items():
        h = v.get("highs") if isinstance(v, dict) else None
        if not h or not isinstance(h.get("objective"), (int, float)): continue
        best[p] = min(best.get(p, float("inf")), h["objective"])
rows = [json.loads(l) for l in open(sys.argv[1]) if l.strip()]
nbad = 0
for r in rows:
    z = best.get(r["problem"]); b = r.get("bound"); o = r.get("objective")
    flag = []
    if z is not None and isinstance(b, (int, float)) and b > z + 1e-6 * max(1, abs(z)): flag.append(f"bound {b} > known {z}")
    if z is not None and r.get("status") == "optimal" and o > z + 1.5e-4 * max(1, abs(z)): flag.append(f"optimal {o} > known {z}")
    if r.get("feasible") is False: flag.append("infeasible x")
    if flag: nbad += 1; print(r["problem"], r["config"], flag)
print("rows", len(rows), "with known value", sum(r["problem"] in best for r in rows), "flagged", nbad)
print("skipped>0:", [(r["problem"], r["config"], r["skipped"]) for r in rows if any(r.get("skipped", []))])

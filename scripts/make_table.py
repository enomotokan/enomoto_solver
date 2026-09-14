import csv
import sys
from pathlib import Path

csv_path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("netlib_benchmark_inplace_switch.csv")
rows = list(csv.DictReader(csv_path.open(encoding="utf-8")))

recs = []
for r in rows:
    if r.get("ours_status") != "optimal" or "kOptimal" not in (r.get("highs_status") or ""):
        continue
    ot = float(r["ours_time"])
    ht = float(r["highs_time"])
    obj_o = float(r["ours_obj"])
    obj_h = float(r["highs_obj"])
    denom = max(abs(obj_h), 1.0)
    obj_rel = abs(obj_o - obj_h) / denom
    recs.append({
        "name": r["name"],
        "n_vars": int(r["n_vars"]),
        "n_rows": int(r["n_rows"]),
        "nnz": int(r["nnz"]),
        "ours": ot,
        "highs": ht,
        "ratio": ot / ht if ht > 0 else float("inf"),
        "obj_rel": obj_rel,
    })

recs.sort(key=lambda x: x["ratio"], reverse=True)

# ---- pretty table ----
hdr = f"{'#':>3} {'problem':<12} {'vars':>6} {'rows':>6} {'nnz':>7} {'HiGHS(s)':>9} {'ours(s)':>9} {'ours/HiGHS':>10} {'obj.rel':>9}"
print(hdr)
print("-" * len(hdr))
for i, r in enumerate(recs, 1):
    print(
        f"{i:>3} {r['name']:<12} {r['n_vars']:>6} {r['n_rows']:>6} {r['nnz']:>7} "
        f"{r['highs']:>9.4f} {r['ours']:>9.4f} {r['ratio']:>9.2f}x {r['obj_rel']:>9.2e}"
    )

# ---- summary ----
print()
tot_o = sum(r["ours"] for r in recs)
tot_h = sum(r["highs"] for r in recs)
wins = sum(1 for r in recs if r["ratio"] <= 1.0)
within2 = sum(1 for r in recs if r["ratio"] <= 2.0)
within3 = sum(1 for r in recs if r["ratio"] <= 3.0)
ratios = sorted(r["ratio"] for r in recs)
mid = ratios[len(ratios) // 2]
print(f"problems: {len(recs)}  (all optimal by both)")
print(f"total time  ours={tot_o:.4f}s  HiGHS={tot_h:.4f}s  ratio={tot_o/tot_h:.2f}x")
print(f"ours faster (<=1.0x): {wins}   within 2x: {within2}   within 3x: {within3}")
print(f"median ratio: {mid:.2f}x   worst: {ratios[-1]:.2f}x ({recs[0]['name']})   best: {ratios[0]:.2f}x ({recs[-1]['name']})")
print(f"max |obj rel diff|: {max(r['obj_rel'] for r in recs):.2e}")

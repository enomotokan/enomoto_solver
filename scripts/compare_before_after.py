import csv
import sys
from pathlib import Path


def load(path):
    d = {}
    for r in csv.DictReader(Path(path).open(encoding="utf-8")):
        if r.get("ours_status") == "optimal" and "kOptimal" in (r.get("highs_status") or ""):
            d[r["name"]] = float(r["ours_time"])
    return d


before_path = sys.argv[1] if len(sys.argv) > 1 else "netlib_after_fixedvar_removal_final.csv"
after_path = sys.argv[2] if len(sys.argv) > 2 else "netlib_benchmark_inplace_switch.csv"

before = load(before_path)
after = load(after_path)
names = sorted(set(before) & set(after))

recs = []
for n in names:
    b, a = before[n], after[n]
    recs.append((n, b, a, a - b, (a / b if b > 0 else float("inf"))))

recs.sort(key=lambda x: x[3])  # most improved first

hdr = f"{'problem':<12} {'before(s)':>10} {'after(s)':>10} {'delta(s)':>10} {'after/before':>12}"
print("=== biggest improvements (after vs before) ===")
print(hdr)
print("-" * len(hdr))
for n, b, a, d, r in recs[:15]:
    print(f"{n:<12} {b:>10.4f} {a:>10.4f} {d:>+10.4f} {r:>11.2f}x")
print()
print("=== biggest regressions ===")
print(hdr)
print("-" * len(hdr))
for n, b, a, d, r in recs[-8:]:
    print(f"{n:<12} {b:>10.4f} {a:>10.4f} {d:>+10.4f} {r:>11.2f}x")

sum_b = sum(before[n] for n in names)
sum_a = sum(after[n] for n in names)
print()
print(f"problems compared: {len(names)}")
print(f"total before={sum_b:.4f}s  after={sum_a:.4f}s  ratio={sum_a/sum_b:.3f}x  delta={sum_a-sum_b:+.4f}s")
improved = [x for x in recs if x[3] < -1e-4]
regressed = [x for x in recs if x[3] > 1e-4]
print(f"improved: {len(improved)}   regressed: {len(regressed)}   unchanged: {len(names)-len(improved)-len(regressed)}")

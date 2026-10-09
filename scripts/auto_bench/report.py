import csv, json, math, sys
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
LIMIT = 600.0
ref = {}
with open(REPO / "benchmarks/paper/per_problem.csv") as f:
    for r in csv.DictReader(f):
        if r["reference_obj"]:
            ref[(r["set"], r["problem"])] = float(r["reference_obj"])

recs = [json.loads(l) for l in Path(sys.argv[1]).read_text().splitlines()]


def ok(rec, solver):
    r = rec["runs"][solver][0]
    if r["status"] != "optimal":
        return False
    other = rec["runs"]["highs" if solver == "enomoto" else "enomoto"][0]
    target = ref.get((rec["set"], rec["problem"]), other.get("obj"))
    if target is None:
        return True
    return abs(r["obj"] - target) <= 1e-6 * max(1.0, abs(target))


def gm(xs, shift=0.0):
    return math.exp(sum(math.log(x + shift) for x in xs) / len(xs)) - shift


rows = []
for s in ["netlib", "kennington", "mittelmann", "all"]:
    rs = [r for r in recs if s == "all" or r["set"] == s]
    if not rs: continue
    t = {k: [r["median"][k] if ok(r, k) else LIMIT for r in rs] for k in ("enomoto", "highs")}
    both = [r for r in rs if ok(r, "enomoto") and ok(r, "highs")]
    tb = {k: [max(r["median"][k], 1e-5) for r in both] for k in ("enomoto", "highs")}
    rows.append((s, len(rs), sum(ok(r, "enomoto") for r in rs), sum(ok(r, "highs") for r in rs),
                 gm([max(x, 1e-5) for x in t["enomoto"]], 10), gm([max(x, 1e-5) for x in t["highs"]], 10),
                 sum(t["enomoto"]), sum(t["highs"]), len(both),
                 gm(tb["enomoto"]) / gm(tb["highs"]) if both else float("nan")))

print("| 集合 | 問題数 | 解けた ENOMOTO | 解けた HiGHS | 10 秒シフト幾何平均 ENOMOTO | 同 HiGHS | 比 | 総時間 ENOMOTO | 総時間 HiGHS | 両方解けた問題の幾何平均比 (ENOMOTO/HiGHS) |")
print("|---|---|---|---|---|---|---|---|---|---|")
for s, n, se, sh, ge, gh, te, th, nb, rb in rows:
    print(f"| {s} | {n} | {se} | {sh} | {ge:.3f}s | {gh:.3f}s | {ge/gh:.2f}x | {te:.1f}s | {th:.1f}s | {rb:.2f}x ({nb} 問) |")

print("\nHiGHS の勝者:", Counter(r["runs"]["highs"][0].get("winner") for r in recs))
print("ENOMOTO の race で IPM が勝った:", [r["problem"] for r in recs if any(e[0] == "race_ipm_crossover_won" for e in r["runs"]["enomoto"][0].get("events", []))])
print("ENOMOTO の race で二段解法が勝った:", [r["problem"] for r in recs if any(e[0].startswith("race_") and e[0] != "race_ipm_crossover_won" for e in r["runs"]["enomoto"][0].get("events", []))])

print("\n解けなかった・目的値の不一致:")
for r in recs:
    for k in ("enomoto", "highs"):
        if not ok(r, k):
            x = r["runs"][k][0]
            print(f"  {r['set']}/{r['problem']} {k}: {x['status']} obj={x.get('obj')} ref={ref.get((r['set'], r['problem']))}")

print("\nKennington / Mittelmann の問題ごと (秒):")
print("| 集合 | 問題 | ENOMOTO | HiGHS | 比 | HiGHS の勝者 |")
print("|---|---|---|---|---|---|")
for r in recs:
    if r["set"] == "netlib": continue
    e, h = r["median"]["enomoto"], r["median"]["highs"]
    fe = f"{e:.3f}" + ("" if ok(r, "enomoto") else " ✗")
    fh = f"{h:.3f}" + ("" if ok(r, "highs") else " ✗")
    print(f"| {r['set']} | {r['problem']} | {fe} | {fh} | {e/h:.2f}x | {r['runs']['highs'][0].get('winner')} |")

print("\nNetlib で比が大きい (ENOMOTO が遅い) 上位 10:")
nl = sorted((r for r in recs if r["set"] == "netlib"), key=lambda r: -r["median"]["enomoto"] / max(r["median"]["highs"], 1e-5))
for r in nl[:10]:
    print(f"  {r['problem']:12s} {r['median']['enomoto']:.4f}s vs {r['median']['highs']:.4f}s ({r['median']['enomoto']/r['median']['highs']:.2f}x)")

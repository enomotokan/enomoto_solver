"""HiGHS (highs.jsonl) が 60 秒で解き、ENOMOTO (bench9・bench10 の各設定) が解けなかった回のある問題を並べる (2026-11-01)。"""
import json, glob
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
B = D.parent / "halve_submip_20261030"
H = {json.loads(l)["problem"]: json.loads(l)["result"] for l in (D / "highs.jsonl").read_text().splitlines()}
hok = lambda r: str(r.get("status", "")).lower() == "optimal" and r.get("time", 99) <= 60
ok = lambda r: r.get("status") == "optimal" and r.get("time", 99) <= 60
E = defaultdict(lambda: defaultdict(list))
for f in sorted(glob.glob(str(B / "bench9_s*.jsonl")) + glob.glob(str(B / "bench10_s*.jsonl"))):
    for l in open(f):
        r = json.loads(l)
        if r["rep"] < 100: E[r["problem"]][r["cfg"]].append(r["result"])
cfgs = ["def", "real", "d2lp1", "trim3", "trim3t20"]
print("HiGHS 解けた", sum(hok(v) for v in H.values()), "/ 58")
print(f"{'問題':26s} {'HiGHS':>7s}  " + "  ".join(f"{c:>12s}" for c in cfgs))
for p, h in sorted(H.items(), key=lambda kv: kv[1].get("time", 99)):
    if not hok(h): continue
    row = {c: E[p][c] for c in cfgs if E[p][c]}
    if all(all(ok(x) for x in v) for v in row.values()): continue
    cell = lambda v: f"{sum(ok(x) for x in v)}/{len(v)}" + (f" {min(x['time'] for x in v if ok(x)):.0f}s" if any(ok(x) for x in v) else "")
    print(f"{p:26s} {h['time']:6.1f}s  " + "  ".join(f"{cell(row[c]) if c in row else '-':>12s}" for c in cfgs))
print("\nENOMOTO (どれかの設定) が解き HiGHS が解けなかった問題:")
for p, h in sorted(H.items()):
    if not hok(h) and any(ok(x) for c in E[p] for x in E[p][c]): print(" ", p, h.get("status"), round(h.get("time", 0), 1))

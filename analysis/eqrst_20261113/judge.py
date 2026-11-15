"""scan.jsonl などを MIPLIB 2017 の既知の値 (scripts/miplib2017-v31.solu の =opt=/=best=) と比べ、再スタート後に EQUIV2 の組を使ったか (ログで
"restart check" の後に "equivalence pass ... used") を出す。使い方: python judge.py X.jsonl"""
import json, sys, gzip
from pathlib import Path
H = Path(__file__).parent; ROOT = H.parents[1]
ref = {}
for l in open(ROOT / "scripts/miplib2017-v31.solu"):
    t = l.split()
    if len(t) >= 3 and t[0] in ("=opt=", "=best="): ref[t[1]] = (t[0], float(t[2]))
nbad = 0
for l in open(sys.argv[1]):
    d = json.loads(l); p = d["problem"]; kind, z = ref.get(p, (None, None)); bad = []
    lg = H / "logs" / f"{p}.{d['config']}.{d['rep']}.log.gz"
    rs = used = 0
    for x in gzip.open(lg, "rt", errors="replace"):
        if "restart check" in x: rs = 1
        if rs and "equivalence pass with implications" in x and x.rstrip().endswith(", used"): used += 1
    if z is not None:
        tol = 1.5e-4 * max(1.0, abs(z)); b = d.get("bound")
        if kind == "=opt=" and isinstance(b, (int, float)) and b > z + tol: bad.append("bound>opt")
        if kind == "=opt=" and d.get("status") == "optimal" and abs(d["objective"] - z) > tol: bad.append("wrong optimal")
        if isinstance(d.get("objective"), (int, float)) and d["objective"] < z - tol and kind == "=opt=": bad.append("obj<opt")
    if d.get("feasible") is False: bad.append(f"infeasible x (row {d.get('row_viol'):.1e})")
    nbad += bool(bad)
    if used or bad or "-v" in sys.argv:
        print(p, d["config"], d["rep"], d.get("status"), d.get("objective"), d.get("bound"), z, f"restart-used {used}" if used else "", bad)
print("bad", nbad)

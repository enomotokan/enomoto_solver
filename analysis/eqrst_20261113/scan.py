"""58 問 (.miplib_cache) を設定 r (再スタート後も EQUIV2) で 10 秒ずつ解き、再スタート後に組を使った問題を探す。
HiGHS の最適値 (benchmarks/highs_ref_4cpu_20261010.jsonl) と比べ、下界が越える・最適で値が違う・点が実行不能を印字する。
使い方: python scan.py OUT.jsonl CONFIG[,CONFIG] [TL]"""
import json, sys, subprocess
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
out = sys.argv[1]; cfgs = sys.argv[2]; tl = sys.argv[3] if len(sys.argv) > 3 else "10"
for mps in sorted((ROOT / ".miplib_cache").glob("*.mps")):
    subprocess.run([sys.executable, str(Path(__file__).parent / "rep.py"), out, mps.stem, tl, "1", cfgs], stdout=subprocess.DEVNULL)
ref = {}
for l in (ROOT / "benchmarks/highs_ref_4cpu_20261010.jsonl").read_text().splitlines():
    d = json.loads(l); h = d["runs"]["highs"][0]
    if h["status"] == "optimal": ref[d["problem"]] = h["obj"]
for l in open(out):
    d = json.loads(l); z = ref.get(d["problem"]); bad = []
    if z is not None:
        tol = 1.5e-4 * max(1.0, abs(z)); b = d.get("bound")
        if isinstance(b, (int, float)) and b > z + tol: bad.append("bound>opt")
        if d.get("status") == "optimal" and abs(d["objective"] - z) > tol: bad.append("wrong optimal")
    if d.get("feasible") is False: bad.append(f"infeasible x (row {d.get('row_viol')})")
    used = [e for e in d["eq"][1:] if e.endswith(", used")]
    print(d["problem"], d["config"], d.get("status"), d.get("objective"), d.get("bound"), z, "restart-used" if used else "", used, bad)

"""55 問 x (既定 / 幅の小さい列の固定を止めたもの)、4 スレッド・10 秒・ENOMOTO_MIP_LOG=1。固定した数、前処理後の大きさ、
目的値と状態を記録する。使い方: python check55.py OUT.jsonl"""
import json, re, sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "psched_ioeq_20261022"))
import run as R
out = Path(sys.argv[1])
ref = {}
for l in (R.ROOT / "benchmarks/highs_ref_4cpu_20261010.jsonl").read_text().splitlines():
    d = json.loads(l); h = d["runs"]["highs"][0]
    if h["status"] == "optimal": ref[d["problem"]] = h["obj"]
CFG = {"default": {}, "notiny": {"ENOMOTO_MIP_NO_TINY_WIDTH_FIX": "1"}}
done = set()
if out.exists():
    done = {(d["problem"], d["config"]) for d in map(json.loads, out.read_text().splitlines()) if d}
for mps in sorted(R.CACHE.glob("*.mps")):
    for c, env in CFG.items():
        if (mps.stem, c) in done:
            continue
        r, err = R.run(mps, 10.0, 4, env, log=True)
        L = err.splitlines()
        tiny = [l.split("MIP: ", 1)[1] for l in L if "columns of width within tolerance" in l]
        pres = [l.split("presolve: ", 1)[1] for l in L if "MIP: presolve: rows" in l]
        skipped = [int(l.split("(skipped ", 1)[1].split()[0]) for l in L if "(skipped " in l]
        z = ref.get(mps.stem); tol = 1.5e-4 * max(1.0, abs(z)) if z is not None else None
        bad = []
        if z is not None:
            b = r.get("bound")
            if isinstance(b, (int, float)) and b > z + tol: bad.append("bound>opt")
            if r.get("status") == "optimal" and abs(r["objective"] - z) > tol: bad.append("wrong optimal")
        if r.get("feasible") is False: bad.append("infeasible x")
        r.update(problem=mps.stem, config=c, tiny=tiny, presolve=pres, skipped=skipped, ref=z, bad=bad,
                 restarts=sum("MIP: restart " in l for l in L))
        with out.open("a") as f:
            f.write(json.dumps(r) + "\n")
        print(mps.stem, c, r.get("status"), r.get("objective"), r.get("bound"), "tiny", tiny[:3], "pres", pres[:1], bad, flush=True)

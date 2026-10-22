"""55 問 x (既定 / EQUIV2+再スタート後も使う)、4 スレッド・10 秒・ログあり。係数の強化で縮めなかった数 (skipped) と、
HiGHS の最適値 (benchmarks/highs_ref_4cpu_20261010.jsonl) との食い違い (下界が最適値を越える・最適と言って値が違う) を見る。
使い方: python check55.py OUT.jsonl [KEY=VAL ...] (追加の環境変数は両方の設定に足す)"""
import json, sys, time
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "psched_ioeq_20261022"))
import run as R
out = Path(sys.argv[1]); extra = dict(a.split("=", 1) for a in sys.argv[2:])
ref = {}
for l in (R.ROOT / "benchmarks/highs_ref_4cpu_20261010.jsonl").read_text().splitlines():
    d = json.loads(l); h = d["runs"]["highs"][0]
    if h["status"] == "optimal": ref[d["problem"]] = h["obj"]
CFG = {"default": {}, "eq2r": {"ENOMOTO_MIP_PROBE_EQUIV2": "1", "ENOMOTO_MIP_PROBE_EQUIV2_RESTART": "1"}}
done = set()
if out.exists():
    done = {(d["problem"], d["config"]) for d in map(json.loads, out.read_text().splitlines()) if d}
for mps in sorted(R.CACHE.glob("*.mps")):
    for c, env in CFG.items():
        if (mps.stem, c) in done:
            continue
        r, err = R.run(mps, 10.0, 4, {**env, **extra}, log=True)
        sk = [int(l.split("(skipped ", 1)[1].split()[0]) for l in err.splitlines() if "(skipped " in l]
        z = ref.get(mps.stem); tol = 1.5e-4 * max(1.0, abs(z)) if z is not None else None
        bad = []
        if z is not None:
            b = r.get("bound")
            if isinstance(b, (int, float)) and b > z + tol: bad.append("bound>opt")
            if r.get("status") == "optimal" and abs(r["objective"] - z) > tol: bad.append("wrong optimal")
            if r.get("feasible") is False: bad.append("infeasible x")
        r.update(problem=mps.stem, config=c, skipped=sk, ref=z, bad=bad,
                 eq=[l.split("probing: ", 1)[1] for l in err.splitlines() if "equivalent binary pairs" in l or "equivalence pass" in l][:6])
        with out.open("a") as f:
            f.write(json.dumps(r) + "\n")
        print(mps.stem, c, r.get("status"), r.get("objective"), r.get("bound"), z, "skipped", sk, bad, flush=True)

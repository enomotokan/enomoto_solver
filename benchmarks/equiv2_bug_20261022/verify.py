"""修正後の確かめ: uccase12 (EQUIV2+再スタート後も使う / e2iohleq+RESTART / 既定) と physiciansched6-2 (e2iohleq) を解く。
使い方: python verify.py OUT.jsonl  (済んだ分は飛ばす。ログは OUT と同じ名前のディレクトリに)"""
import json, sys, time
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "psched_ioeq_20261022"))
import run as R
out = Path(sys.argv[1]); logd = out.with_suffix(""); logd.mkdir(parents=True, exist_ok=True)
EQ2R = {"ENOMOTO_MIP_PROBE_EQUIV2": "1", "ENOMOTO_MIP_PROBE_EQUIV2_RESTART": "1"}
JOBS = [("uccase12", "eq2restart", EQ2R, 45, k) for k in range(3)] \
     + [("uccase12", "e2iohleq_restart", {**R.CONFIGS["e2iohleq"], "ENOMOTO_MIP_PROBE_EQUIV2_RESTART": "1"}, 60, k) for k in range(3)] \
     + [("uccase12", "default", {}, 60, k) for k in range(3)] \
     + [("physiciansched6-2", "e2iohleq", R.CONFIGS["e2iohleq"], 60, k) for k in range(2)]
done = set()
if out.exists():
    done = {(d["problem"], d["config"], d["rep"]) for d in map(json.loads, out.read_text().splitlines()) if d}
for prob, cfg, env, tl, k in JOBS:
    if (prob, cfg, k) in done:
        continue
    r, err = R.run(R.CACHE / f"{prob}.mps", tl, 4, env, log=True)
    (logd / f"{prob}_{cfg}_{k}.log").write_text(err)
    r.update(problem=prob, config=cfg, rep=k, wall=time.strftime("%H:%M:%S"),
             eq=[l.split("probing: ", 1)[1] for l in err.splitlines() if "equivalence pass" in l or "equivalent binary pairs" in l],
             skipped=[l.split("presolve: ", 1)[1] for l in err.splitlines() if "skipped" in l and "redundant within tolerance" in l and "skipped 0 " not in l],
             msgs=[l for l in err.splitlines() if "closed the gap" in l or "WARNING" in l][:3])
    with out.open("a") as f:
        f.write(json.dumps(r) + "\n")
    print(json.dumps({x: r.get(x) for x in ("problem", "config", "rep", "status", "objective", "bound", "root_bound", "time", "eq", "skipped")}), flush=True)

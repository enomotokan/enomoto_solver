"""uccase12 を EQUIV2 (+再スタート後も使う) で繰り返し解き、再スタート後の問題とプロービングの入力・同値の組を書き出す。
使い方: python rep.py OUTDIR N [TL] [THREADS] [KEY=VAL ...]  (再スタート後に組を使った回で止める)"""
import json, sys, shutil
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "psched_ioeq_20261022"))
import run as R
out = Path(sys.argv[1]).resolve(); out.mkdir(parents=True, exist_ok=True)
n = int(sys.argv[2]); tl = float(sys.argv[3]) if len(sys.argv) > 3 else 45.0
th = int(sys.argv[4]) if len(sys.argv) > 4 else 4
extra = dict(a.split("=", 1) for a in sys.argv[5:])
for k in range(n):
    d = out / f"rep{k}"; d.mkdir(exist_ok=True)
    env = {"ENOMOTO_MIP_PROBE_EQUIV2": "1", "ENOMOTO_MIP_PROBE_EQUIV2_RESTART": "1",
           "ENOMOTO_MIP_DEBUG_DUMP_PROBE": str(d / "probe"), "ENOMOTO_MIP_DEBUG_DUMP_RESTART": str(d / "restart"), **extra}
    r, err = R.run(R.CACHE / "uccase12.mps", tl, th, env, log=True)
    (d / "log.txt").write_text(err)
    used = [l for l in err.splitlines() if "equivalence pass" in l]
    r["rep"] = k; r["eq"] = used
    print(json.dumps({x: r.get(x) for x in ("rep", "status", "objective", "bound", "root_bound", "time", "eq")}), flush=True)
    with (out / "res.jsonl").open("a") as f: f.write(json.dumps(r) + "\n")
    if any(l.endswith(", used") for l in used[1:]):
        break

"""止めの合図から別スレッドのサブ MIP が止まるまでの秒を測る (analysis/timedep_20261114.md の 9.4 節)。
python halt_lat.py OUT.jsonl 問題:秒,... [種]。束 CAND1115 (TIMEDEP Et2 入り) で、SUBMIP_HALT なし (off) とあり (on) を 4 スレッドで解き、
ログの "stopped Xs after the stop request" を集める。ログは logs_halt/。"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import importlib.util
sp = importlib.util.spec_from_file_location("cfg1115", WT / "analysis/cand1115_20261115/cfg.py"); m = importlib.util.module_from_spec(sp); sp.loader.exec_module(m)
import run_miplib_benchmark as rb  # noqa: E402
out = Path(sys.argv[1]); jobs = [x.split(":") for x in sys.argv[2].split(",")]; seed = sys.argv[3] if len(sys.argv) > 3 else "1"
L = D / "logs_halt"; L.mkdir(exist_ok=True)
for p, tl in jobs:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for mode in ("off", "on"):
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(m.CAND1115); env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": seed, "ENOMOTO_MIP_LOG": "1"})
        if mode == "on": env["ENOMOTO_MIP_SUBMIP_HALT"] = "1"
        lf = L / f"{p}_{mode}_s{seed}.log"
        with open(lf, "w") as f:
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", tl, "--threads", "4"], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=f, text=True)
        res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        lat = [(mm.group(1), float(mm.group(2))) for mm in re.finditer(r"parallel sub-MIP \(([^)]*)\): status .*?\(stopped ([\d.]+)s after the stop request\)", lf.read_text())]
        row = {"problem": p, "mode": mode, "seed": seed, "status": res.get("status"), "time": res.get("time"), "objective": res.get("objective"), "latency": lat}
        with open(out, "a") as f: f.write(json.dumps(row) + "\n")
        print(row, flush=True)

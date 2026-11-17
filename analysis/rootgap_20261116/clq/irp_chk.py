"""irp で sig が遅くなる元を見る (README の 5 節): c18・sig・sig の f を小さくした形を 4 スレッド・600 秒で種 1・2 に解き、根の clique の本数・
根のカットのループの時間と下界をログ (ENOMOTO_MIP_LOG) から拾って irp_chk.jsonl に足す。python irp_chk.py"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[2]
sys.path.insert(0, str(D)); from cfg import CFGS, C18  # noqa: E402
V = {"c18": CFGS["c18"], "sig": CFGS["c18_sig"],
     "sig_b07": {**CFGS["c18_sig"], "ENOMOTO_T_MIP_CLIQUE_SIG_B": "0.7"},   # f 約 0.37
     "sig_b10": {**CFGS["c18_sig"], "ENOMOTO_T_MIP_CLIQUE_SIG_B": "1.0"}}   # f 約 0.21
mps = WT / ".miplib_cache" / "irp.mps"
for seed in (1, 2):
    for name, cf in V.items():
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(cf); env["ENOMOTO_MIP_THREADS"] = "4"; env["ENOMOTO_T_MIP_SEED"] = str(seed); env["ENOMOTO_MIP_LOG"] = "1"
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", "4"],
                            cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, errors="replace")
        r = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        err = pr.stderr.splitlines()
        f = next((l for l in err if "clique sigmoid" in l), "")
        done = [l for l in err if "cut loop done" in l]
        rounds = [l for l in err if re.search(r"MIP: cut round \d+:", l)]
        row = {"v": name, "seed": seed, "status": r.get("status"), "time": round(r.get("time") or 0, 1), "nodes": r.get("nodes"),
               "root_bound": r.get("root_bound"), "f": f[-60:], "loop_done": [d[-90:] for d in done[:2]], "nrounds": len(rounds),
               "last_round": rounds[-1][-120:] if rounds else ""}
        with open(D / "irp_chk.jsonl", "a") as fo: fo.write(json.dumps(row) + "\n")
        print(row, flush=True)

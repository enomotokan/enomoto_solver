"""irp の前処理の差 (README の 4 節の項目 3): HiGHS が消す 945 列はどれも同じ列 (双子、費用も境界も同じ 0-1 列)。当方の試作の鍵
ENOMOTO_PRESOLVE_PARALLEL_INT (前処理の平行列の併合を整数列にも)・ENOMOTO_MIP_SYM_TWINS (対称性の前に双子を併合) を c18 に足し、
4 スレッド・600 秒・種 1・2 で解いて irp_twin.jsonl に足す (前処理の後の列の数と根の下界をログから拾う)。python irp_twin.py"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(D / "clq")); from cfg import C18  # noqa: E402
V = {"c18": {}, "parint": {"ENOMOTO_PRESOLVE_PARALLEL_INT": "1"}, "twins": {"ENOMOTO_MIP_SYM_TWINS": "1"}}
for seed in (1, 2):
    for name, ex in V.items():
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(C18); env.update(ex); env["ENOMOTO_MIP_THREADS"] = "4"; env["ENOMOTO_T_MIP_SEED"] = str(seed); env["ENOMOTO_MIP_LOG"] = "1"
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache/irp.mps"), "--time-limit", "600", "--threads", "4"],
                            cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, errors="replace")
        r = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        err = pr.stderr.splitlines()
        pre = next((l for l in err if "MIP: presolve: rows" in l), "")
        tw = next((l for l in err if "twins" in l), "")
        done = next((l for l in err if "cut loop done" in l), "")
        row = {"v": name, "seed": seed, "status": r.get("status"), "time": round(r.get("time") or 0, 1), "nodes": r.get("nodes"), "root_bound": r.get("root_bound"),
               "presolve": pre[-150:], "twins": tw[-160:], "loop1": done[-60:]}
        with open(D / "irp_twin.jsonl", "a") as fo: fo.write(json.dumps(row) + "\n")
        print(row, flush=True)

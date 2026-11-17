"""nw04 で twins (ENOMOTO_MIP_SYM_TWINS) が 1.2 倍前後遅い理由を見る (README の 6 節): c18 と twins を 4 スレッド・60 秒・種 1・2 で解き、
双子の数・根のカットのラウンドごとの目的値・時間の内訳をログから拾って nw04_twin.jsonl に足す。python nw04_twin.py"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[2]
sys.path.insert(0, str(D)); from cfg import CFGS  # noqa: E402
for seed in (1, 2):
    for name in ("c18", "c18_twins"):
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(CFGS[name]); env["ENOMOTO_MIP_THREADS"] = "4"; env["ENOMOTO_T_MIP_SEED"] = str(seed); env["ENOMOTO_MIP_LOG"] = "1"
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache/nw04.mps"), "--time-limit", "60", "--threads", "4"],
                            cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, errors="replace")
        r = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        err = pr.stderr.splitlines()
        objs = [re.search(r"obj (\S+) \(([\d.]+)s\)", l) for l in err if re.search(r"MIP: cut round \d+: \d+ cuts", l)]
        tw = next((l for l in err if "twins" in l), "")
        root_end = next((l for l in err if "root done" in l or "root node" in l.lower()), "")
        first_inc = next((l for l in err if "new incumbent" in l.lower() or "incumbent" in l.lower()), "")
        row = {"v": name, "seed": seed, "status": r.get("status"), "time": round(r.get("time") or 0, 1), "nodes": r.get("nodes"), "root_bound": r.get("root_bound"),
               "twins": tw[-150:], "round_objs": [(m.group(1), m.group(2)) for m in objs if m][:12], "first_inc": first_inc[-120:]}
        with open(D / "nw04_twin.jsonl", "a") as fo: fo.write(json.dumps(row) + "\n")
        print(row, flush=True)

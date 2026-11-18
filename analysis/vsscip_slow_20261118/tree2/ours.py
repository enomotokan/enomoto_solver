"""群 2: 当方を環境変数つきで 1 回解き、結果を ours.jsonl に足す。ログ (ENOMOTO_MIP_LOG=1) は logs/ に出す。

使い方: python ours.py 問題 名前 [--tl 秒] [--threads N] [--seed N] [--log] -- 環境変数=値 ..."""
import json, os, subprocess, sys, time, argparse, gzip
from pathlib import Path

HERE = Path(__file__).resolve().parent
WT = HERE.parents[2]

def main():
    argv = sys.argv[1:]
    envs = []
    if "--" in argv:
        i = argv.index("--"); envs = argv[i + 1:]; argv = argv[:i]
    ap = argparse.ArgumentParser()
    ap.add_argument("prob"); ap.add_argument("name"); ap.add_argument("--tl", type=float, default=600)
    ap.add_argument("--threads", type=int, default=4); ap.add_argument("--seed", type=int, default=1); ap.add_argument("--log", action="store_true")
    a = ap.parse_args(argv)
    env = dict(os.environ)
    for kv in envs:
        k, v = kv.split("=", 1); env[k] = v
    env["ENOMOTO_T_MIP_SEED"] = str(a.seed)
    env["ENOMOTO_MIP_THREADS"] = str(a.threads)
    if a.log: env["ENOMOTO_MIP_LOG"] = "1"
    mps = WT / ".miplib_cache" / f"{a.prob}.mps"
    (HERE / "logs").mkdir(exist_ok=True)
    lp = HERE / "logs" / f"ours_{a.prob}_{a.name}_s{a.seed}_t{a.threads}.log"
    t = time.perf_counter()
    with open(lp, "w") as lf:
        p = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(a.tl), "--threads", str(a.threads)],
                           env=env, stdout=subprocess.PIPE, stderr=lf, text=True)
    wall = time.perf_counter() - t
    try:
        res = json.loads(p.stdout.strip().splitlines()[-1])
    except Exception:
        res = {"status": "crash", "rc": p.returncode, "out": p.stdout[-300:]}
    r = {"problem": a.prob, "name": a.name, "env": envs, "seed": a.seed, "threads": a.threads, "tl": a.tl, "result": res, "wall": round(wall, 1)}
    print(json.dumps(r), flush=True)
    with open(HERE / "ours.jsonl", "a") as f: f.write(json.dumps(r) + "\n")

if __name__ == "__main__":
    main()

"""CUTMIX=z の計測の前に、M の問題ごとに z が働くか (根の intsmall ≥ 0.5) を調べる: python zscan.py [秒=90] [並べる数=4]。
1 スレッド・種 1・ENOMOTO_MIP_CUTMIX=z・ENOMOTO_MIP_LOG=1 で解き始め、最初の `CUTMIX types` の行 (根のカットのループの初め、前処理の後の問題)
が出たら止める。出ないまま秒が尽きた問題は intsmall 不明 (null)。zscan.jsonl に足す (途中から再開できる)。"""
import json, os, subprocess, sys, time, re, select
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
tl = float(sys.argv[1]) if len(sys.argv) > 1 else 90.0; par = int(sys.argv[2]) if len(sys.argv) > 2 else 4
sh = json.loads((WT / "analysis/vsscip_20261120/shards_M.json").read_text())["shards"]
probs = sorted({t[0] for s in sh for t in s})
out = D / "zscan.jsonl"
done = {json.loads(l)["problem"] for l in open(out)} if out.exists() else set()
def run(p):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_CUTMIX": "z"})
    t0 = time.time()
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", str(tl), "--threads", "1"],
                          cwd=WT, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    row = {"problem": p, "intsmall": None}
    for l in pr.stderr:
        if l.startswith("CUTMIX types"):
            row.update({k: float(v) for k, v in re.findall(r"(\w+) ([0-9.]+)", l) if k in ("intsmall", "introw", "setp", "knap", "cont", "m", "n")})
            break
    pr.kill(); pr.wait()
    row["sec"] = round(time.time() - t0, 1)
    return row
with ThreadPoolExecutor(par) as ex:
    for row in ex.map(run, [p for p in probs if p not in done]):
        with open(out, "a") as f: f.write(json.dumps(row) + "\n")
        print(row["problem"], row["intsmall"], row["sec"], flush=True)

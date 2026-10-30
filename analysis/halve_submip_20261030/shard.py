"""分担表 (shards.json) の k 番目を解き、bench_s<k>.jsonl に足す (2026-10-30。別のクラウドセッションで並列に回す)。
使い方: python analysis/halve_submip_20261030/shard.py <k>。問題は MIPLIB から .miplib_cache/ に取得する。
設定は run.py の CFGS (4 スレッド・60 秒)。問題ごとに設定の順を回で逆にする。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from run import CFGS  # noqa: E402
k = int(sys.argv[1])
S = json.loads((D / "shards.json").read_text())
OUT = D / f"bench_s{k}.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
for p, rep in S["shards"][k]:
    mps = rb.fetch(p, WT / ".miplib_cache")
    cfgs = S["cfgs"] if rep % 2 == 0 else S["cfgs"][::-1]
    for c in cfgs:
        if (p, c, rep) in done:
            continue
        env = dict(os.environ, ENOMOTO_MIP_THREADS="4", **CFGS[c])
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                                "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=300)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": str(e)[:300]}
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "result": res, "shard": k, "nproc": os.cpu_count(), "at": time.strftime("%H:%M:%S")}) + "\n")
        print(p, c, rep, res.get("status"), res.get("time"), flush=True)

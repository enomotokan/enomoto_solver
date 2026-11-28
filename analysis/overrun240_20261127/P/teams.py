"""10teams の 2 スレッドの揺れの確かめ: PYTHONPATH=/home/user/es_bg/python python teams.py → res_teams.jsonl に足す (途中から再開できる)。
P (60 秒) の i_t2 で 10teams が全種で遅れた (README 9 節) ので、種 1〜8 × {既定, POLISH_BG=i} を 2 スレッドで回す。版は P と同じ f7c088a80。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
OUT = D / "res_teams.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["cfg"], r["seed"]))
CFGS = {"def_t2": {}, "i_t2": {"ENOMOTO_MIP_POLISH_BG": "i"}}
for seed in range(1, 9):
    for c, ev in CFGS.items():
        if (c, seed) in done:
            continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update({"ENOMOTO_MIP_THREADS": "2", "ENOMOTO_T_MIP_SEED": str(seed), **ev})
        t0 = time.time()
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache/10teams.mps"), "--time-limit", "60", "--threads", "2"],
                            cwd=WT, env=env, capture_output=True, text=True, timeout=400)
        res = [l for l in pr.stdout.splitlines() if l.startswith("{")]
        result = json.loads(res[-1]) if res else {"status": "crash"}
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": "10teams", "cfg": c, "seed": seed, "result": result, "wall": round(time.time() - t0, 1)}) + "\n")
        print(c, seed, result.get("status"), result.get("time"), result.get("nodes"), flush=True)

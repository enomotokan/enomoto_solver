"""S5 の試作の害を 58 問 (analysis/cbcimpl_20261029/probs.py、4 スレッド・60 秒) で見る。問題ごとに設定を順に回し、回ごとに順を逆にする。
使い方: python bench58.py 出力.jsonl 回の番号,... 設定名 ...   (設定は下の CFGS。途中から再開できる)"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
sys.path.insert(0, str(WT / "scripts"))
from probs import PROBS  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "trforce": {"ENOMOTO_MIP_TREE_RESTART": "force"}}
out = Path(sys.argv[1]); reps = [int(x) for x in sys.argv[2].split(",")]; names = sys.argv[3:]
done = set()
if out.exists():
    for l in out.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
for rep in reps:
    order = names if rep % 2 == 0 else names[::-1]
    for p in PROBS:
        for cfg in order:
            if (p, cfg, rep) in done: continue
            mps = rb.fetch(p, WT / ".miplib_cache")
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(rep), **CFGS[cfg])
            t0 = time.time()
            try:
                r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                                   cwd=WT, env=env, capture_output=True, text=True, timeout=300)
                res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[:300]}
            with open(out, "a") as f:
                f.write(json.dumps({"problem": p, "cfg": cfg, "rep": rep, "wall": round(time.time() - t0, 1), "result": res}) + "\n")
            print(p, cfg, rep, res.get("status"), res.get("time"), flush=True)

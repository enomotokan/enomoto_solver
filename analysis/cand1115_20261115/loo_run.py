"""1 つ外しの分担を回す: python loo_run.py K N (PROBS × 種 1〜4 の組を N に分け、K 番目を 4 スレッド・60 秒で。res_L_sK.jsonl)。
同じ (問題, 種) の 9 設定は同じ分担で続けて解き、順は種で回す。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from loo_cfg import CFGS, PROBS  # noqa: E402
K, N = int(sys.argv[1]), int(sys.argv[2])
pairs = [(p, s) for p in PROBS for s in (1, 2, 3, 4)][K::N]
names = list(CFGS)
OUT = D / f"res_L_s{K}.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip(): r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))
for p, s in pairs:
    mps = rb.fetch(p, WT / ".miplib_cache")
    order = names[s % len(names):] + names[:s % len(names)]
    for c in order:
        if (p, c, s) in done: continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(CFGS[c]); env["ENOMOTO_MIP_THREADS"] = "4"; env["ENOMOTO_T_MIP_SEED"] = str(s)
        t0 = time.time()
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                            cwd=WT, env=env, capture_output=True, text=True, timeout=900)
        try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "seed": s, "tl": 60, "result": res, "wall": round(time.time() - t0, 1), "shard": K}) + "\n")
        print(p, s, c, res.get("status"), res.get("time"), flush=True)

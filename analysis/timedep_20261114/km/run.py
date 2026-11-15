"""分担を回す: python run.py K (8 問 × 種 1・2 の 16 組を 8 分担に分け (1 分担 2 組 × 2 設定 = 4 回、最長 40 分)、K 番目を 4 スレッド・600 秒で。
res_sK.jsonl に足す、途中から再開できる)。同じ (問題, 種) の 3 設定は同じ分担で続けて解き、順は種で回す。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CFGS, PROBS  # noqa: E402
K = int(sys.argv[1])
pairs = [(p, s) for p in PROBS for s in (1, 2)][K::8]
names = list(CFGS)
OUT = D / f"res_s{K}.jsonl"
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip(): r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))
for p, s in pairs:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for c in names[s % len(names):] + names[:s % len(names)]:
        if (p, c, s) in done: continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(CFGS[c]); env["ENOMOTO_MIP_THREADS"] = "4"; env["ENOMOTO_T_MIP_SEED"] = str(s)
        t0 = time.time()
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", "4"],
                            cwd=WT, env=env, capture_output=True, text=True, timeout=1800)
        try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "seed": s, "tl": 600, "result": res, "wall": round(time.time() - t0, 1), "shard": K}) + "\n")
        print(p, s, c, res.get("status"), res.get("time"), flush=True)

"""nw04・nu25-pr12 の遅れの切り分け (9.8 節): 既定・REFAC_BAL=k・kr を 1・4 スレッド、種 1〜3 で回し、
結果と `ENOMOTO_DEBUG_TICKSUM` の行 (主のスレッドの分解の数・ticks の部分ごとの数、lpit は LP の反復) を nwcut.jsonl に書く。
途中から再開できる。python nwcut.py [問題 ...]"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
OUT = D / "nwcut.jsonl"
probs = sys.argv[1:] or ["nw04", "nu25-pr12"]
CF = {"def": {}, "k": {"ENOMOTO_LU_REFAC_BAL": "k"}, "kr": {"ENOMOTO_LU_REFAC_BAL": "kr"}}
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["th"], r["seed"]))
for p in probs:
    for th in ("1", "4"):
        for seed in (1, 2, 3):
            for c, e in CF.items():
                if (p, c, th, seed) in done: continue
                env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
                env.update(e); env.update({"ENOMOTO_MIP_THREADS": th, "ENOMOTO_T_MIP_SEED": str(seed), "ENOMOTO_DEBUG_TICKSUM": "1"})
                t0 = time.time()
                pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                                     "--time-limit", "60", "--threads", th], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
                except Exception: res = {"status": "crash"}  # noqa: BLE001
                ts = [l for l in pr.stderr.splitlines() if l.startswith("TICKSUM")]
                tk = {}
                if ts:
                    for w in ts[-1].split()[1:]:
                        a, b = w.split("="); tk[a] = float(b)
                with open(OUT, "a") as f:
                    f.write(json.dumps({"problem": p, "cfg": c, "th": th, "seed": seed, "result": res, "ticksum": tk, "wall": round(time.time() - t0, 1)}) + "\n")
                print(p, c, th, seed, res.get("status"), res.get("time"), res.get("nodes"), tk.get("builds"), tk.get("lpit"), flush=True)

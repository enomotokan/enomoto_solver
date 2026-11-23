"""2 つのビルド (wheel) を同じ問題で交互に比べる (結果を変えない直しの確かめ: ノード・目的値が同じで時間が減るか)。
python cmp.py 札 OLD.whl NEW.whl [回数] → cmp_<札>.jsonl。各回で old → new の順を入れ替え、そのたびに入れ直す。計時は使わない (ENOMOTO_ は消す)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
RUNS = [("mas76", 1, 120, {}), ("mas76", 4, 120, {}), ("pk1", 1, 120, {}), ("pk1", 4, 120, {}), ("noswot", 1, 120, {}), ("noswot", 4, 120, {}),
        ("fhnw-binpack4-4", 1, 120, {"ENOMOTO_MIP_NODE_CLEANUP": "q"}), ("fhnw-binpack4-4", 4, 120, {"ENOMOTO_MIP_NODE_CLEANUP": "q"})]
tag, whls = sys.argv[1], {"old": sys.argv[2], "new": sys.argv[3]}
reps = int(sys.argv[4]) if len(sys.argv) > 4 else 2
OUT = D / f"cmp_{tag}.jsonl"
done = {(r["rep"], r["build"], r["problem"], r["threads"]) for r in map(json.loads, OUT.open())} if OUT.exists() else set()
for rep in range(reps):
    order = ["old", "new"] if rep % 2 == 0 else ["new", "old"]
    for b in order:
        todo = [r for r in RUNS if (rep, b, r[0], r[1]) not in done]
        if not todo: continue
        subprocess.run([sys.executable, "-m", "pip", "install", "-q", "--force-reinstall", "--no-deps", whls[b]], check=True)
        for prob, th, tl, extra in todo:
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(extra); env.update(ENOMOTO_MIP_THREADS=str(th), ENOMOTO_T_MIP_SEED="1")
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{prob}.mps"),
                                 "--time-limit", str(tl), "--threads", str(th)], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
            try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
            except Exception: res = {"status": "crash"}  # noqa: BLE001
            with open(OUT, "a") as f:
                f.write(json.dumps({"rep": rep, "build": b, "problem": prob, "threads": th, "env": extra, "result": res, "at": time.strftime("%H:%M:%S")}) + "\n")
            print(rep, b, prob, th, res.get("status"), res.get("nodes"), res.get("time"), flush=True)

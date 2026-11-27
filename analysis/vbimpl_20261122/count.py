"""VB_IMPL が足す変数上下限の数と、根の下界 (VB_IMPL=1 と既定)。根だけ (ENOMOTO_MIP_NODE_LIMIT=1)、4 スレッド・種 1・120 秒。
python count.py → count.jsonl・logs/count_<問題>_<設定>.log"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
PROBS = ["unitcal_7", "hypothyroid-k1", "mzzv11", "supportcase33", "mcsched", "tr12-30", "neos-1456979", "neos-4738912-atrato", "fastxgemm-n2r6s0t2",
         "neos-933966", "timtab1", "neos-3627168-kasai", "rocI-4-11", "h80x6320d", "traininstance6", "neos-960392", "qnet1", "neos17"]
OUT = D / "count.jsonl"
done = {(r["problem"], r["cfg"]) for r in map(json.loads, OUT.open())} if OUT.exists() else set()
for p in PROBS:
    for cfg, ex in (("def", {}), ("vb", {"ENOMOTO_MIP_VB_IMPL": "1"})):
        if (p, cfg) in done: continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ex); env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_NODE_LIMIT="1")
        log = D / "logs" / f"count_{p}_{cfg}.log"
        with open(log, "w") as fe:
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", "120", "--threads", "4"],
                                cwd=WT, env=env, stdout=subprocess.PIPE, stderr=fe, text=True)
        try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash"}  # noqa: BLE001
        m = re.search(r"VB_IMPL: (\d+) variable bounds from (\d+)", log.read_text())
        rec = {"problem": p, "cfg": cfg, "added": int(m.group(1)) if m else None, "impl": int(m.group(2)) if m else None,
               "root_bound": res.get("root_bound"), "status": res.get("status"), "time": res.get("time")}
        with open(OUT, "a") as f: f.write(json.dumps(rec) + "\n")
        print(rec, flush=True)

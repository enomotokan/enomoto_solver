"""RELPS の第 1 段: 5 問 (mcsched・swath1・dano3_5・30n20b8 (悪化しないこと)・mc11) を、既定 (def) と試作 ENOMOTO_MIP_RELPS の文字の組
(RUN_CFGS、既定 def,u,ucr) で、1・4 スレッド (RUN_TH)・種 1・上限 RUN_TL 秒 (既定 300) で解く。
どの回も ENOMOTO_MIP_RELPS_DIAG=1 で「MIP: RELPS:」の数え (強分枝をした選び・強分枝していない信頼できない列を選んだ数など) と
「MIP: branching:」の行を拾う (4 スレッドは主のスレッドの行)。python run5.py [問題 ...] → res5.jsonl (途中から再開できる)、logs5/"""
import gzip, json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
OUT = D / "res5.jsonl"; LOGS = D / "logs5"; LOGS.mkdir(exist_ok=True)
TL = int(os.environ.get("RUN_TL", "300"))
done = {(r["problem"], r["th"], r["cfg"]) for r in map(json.loads, OUT.read_text().splitlines())} if OUT.exists() else set()
PROBS = sys.argv[1:] or ["swath1", "mc11", "30n20b8", "mcsched", "dano3_5"]
for th in map(int, os.environ.get("RUN_TH", "1,4").split(",")):
    for p in PROBS:
        for cfg in os.environ.get("RUN_CFGS", "def,u,ucr").split(","):
            if (p, th, cfg) in done: continue
            mps = rb.fetch(p, WT / ".miplib_cache")
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update({"ENOMOTO_MIP_THREADS": str(th), "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_RELPS_DIAG": "1"})
            if cfg != "def": env["ENOMOTO_MIP_RELPS"] = cfg
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", str(th)],
                                cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=TL + 600)
            try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
            except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
            keep = [l for l in pr.stderr.splitlines() if "RELPS" in l or "branching:" in l or l.startswith("TRSTAT: tree start")]
            (LOGS / f"{p}_t{th}_{cfg}.log.gz").write_bytes(gzip.compress(("\n".join(keep) + "\n").encode()))
            rel = [list(map(int, re.findall(r"\d+", l.split("RELPS:")[1]))) for l in keep if "MIP: RELPS:" in l]
            br = [list(map(int, m.groups())) for m in (re.search(r"branching: (\d+) selections, (\d+) without.*sb LPs (\d+) iters (\d+)", l) for l in keep) if m]
            r = {"problem": p, "th": th, "cfg": cfg, "tl": TL, "res": {k: res.get(k) for k in ("status", "objective", "bound", "nodes", "time", "root_bound")}, "relps": rel, "branching": br}
            with open(OUT, "a") as f: f.write(json.dumps(r) + "\n")
            print(json.dumps({k: r[k] for k in ("problem", "th", "cfg")} | r["res"] | {"relps": rel[-1:] , "sb": [b[2:] for b in br[-1:]]}), flush=True)
print("RUN5_DONE", flush=True)

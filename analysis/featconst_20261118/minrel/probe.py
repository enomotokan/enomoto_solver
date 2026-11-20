"""CUTSEL_MIN_REL の逆向きが今の既定の上でまだあるかの測り直し (featconst BRIEF の 2、段階 (1))。
python probe.py [種] : 効く側 4 問・損する側 3 問 × {def, mr01 (0.1), mr0 (0)} を 4 スレッド・120 秒で解き、probe.jsonl に足す。
def の回は ENOMOTO_MIP_LOG・DEBUG_CUTSEL の標準エラーを logs/ に gzip で残す (特徴を数えるため)。途中から再開できる。"""
import gzip, json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
SEED = int(sys.argv[1]) if len(sys.argv) > 1 else 1
TL = 120
CF = {"def": {}, "mr01": {"ENOMOTO_T_CUTSEL_MIN_REL": "0.1"}, "mr0": {"ENOMOTO_T_CUTSEL_MIN_REL": "0"}}
PROBS = ["roll3000", "sp150x300d", "nu25-pr12", "unitcal_7", "mik-250-20-75-4", "misc07", "p200x1188c"]
OUT = D / "probe.jsonl"
done = {(r["problem"], r["cfg"], r["seed"]) for r in map(json.loads, OUT.read_text().splitlines())} if OUT.exists() else set()
(D / "logs").mkdir(exist_ok=True)
for p in PROBS:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for c in CF:
        if (p, c, SEED) in done: continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(CF[c]); env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": str(SEED)})
        if c == "def": env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_DEBUG_CUTSEL": "1"})
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                            cwd=WT, env=env, capture_output=True, text=True)
        try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
        if c == "def":
            (D / "logs" / f"{p}_s{SEED}.log.gz").write_bytes(gzip.compress(pr.stderr.encode()))
        r = {"problem": p, "cfg": c, "seed": SEED, "tl": TL, "result": res}
        print(p, c, SEED, res.get("status"), res.get("objective"), round(res.get("time", 0) or 0, 1), res.get("root_bound"), flush=True)
        with OUT.open("a") as f: f.write(json.dumps(r) + "\n")

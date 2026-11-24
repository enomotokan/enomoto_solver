"""診断 ENOMOTO_MIP_DIAG_GLOBALAUDIT の数を集める: python analysis/globalaudit_20261124/run.py [秒] [スレッド] [種]
問題は analysis/cbcimpl_20261029/probs.py の 58 問のうち下の PROBS。ENOMOTO_MIP_LOG=1 の標準エラーから GLOBALAUDIT の行を取り、
res_t{スレッド}.jsonl に足す (同じ (問題, スレッド, 種) は飛ばす)。解き方は既定のまま (診断は数えて出すだけ)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
PROBS = ['fiber', '10teams', 'qnet1', 'dcmulti', 'misc07', 'khb05250', 'p200x1188c', 'neos17', 'mas76', 'eil33-2',
         'air03', 'rout', 'gt2', 'blend2', 'p0201', 'noswot', 'timtab1', 'neos5']
TL = float(sys.argv[1]) if len(sys.argv) > 1 else 60.0
TH = sys.argv[2] if len(sys.argv) > 2 else "4"
SEED = sys.argv[3] if len(sys.argv) > 3 else "1"
OUT = D / f"res_t{TH}.jsonl"
done = set()
if OUT.exists():
    done = {(r["problem"], r["seed"]) for r in map(json.loads, OUT.read_text().splitlines())}
for p in PROBS:
    if (p, SEED) in done:
        continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_DIAG_GLOBALAUDIT": "1", "ENOMOTO_MIP_THREADS": TH, "ENOMOTO_T_MIP_SEED": SEED})
    t0 = time.time()
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", TH],
                        cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 300)
    try:
        res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {"status": "crash"}
    ga = [l for l in pr.stderr.splitlines() if "GLOBALAUDIT" in l]
    rst = [l for l in pr.stderr.splitlines() if "restart" in l.lower() and l.startswith("MIP:")][:20]
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "seed": SEED, "threads": TH, "tl": TL, "result": res, "ga": ga, "restart_lines": rst, "wall": round(time.time() - t0, 1)}) + "\n")
    print(p, res.get("status"), res.get("time"), len(ga), flush=True)

"""DUALEQ2・STALLSCALE・POOL_TAB の計測: python run.py OUT.jsonl SEEDS (例 1,2) [設定,...]。設定を与えなければ以下の既定の組。58 問 (analysis/cand1114_20261113 の P の組) × 種 × {c14, c14u} を 60 秒・4 スレッドで、
DUALEQ が前処理を変える 4 問 (binkar10_1・beavma・flugpl・neos-960392) は c14ud・c14dg も足す。同じ (問題, 種) は続けて回す。途中から再開できる。
c14 = cand1114、c14u = + ENOMOTO_MIP_STALL_SCALE=u、c14ud = c14u + DUALEQ、c14dg = c14 + DUALEQ + ENOMOTO_T_MIP_DUALEQ_MINRED=0.01。
設定を与えたとき (例 c14x,c14dx) は 58 問すべてをその設定で回す。c14x = c14 + ENOMOTO_MIP_POOL_TAB=x、c14dx = c14x + DUALEQ、c14r = c14 の再計測 (同じ時期の対照)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis/cand1114_20261113"))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CAND1114  # noqa: E402
CFGS = {"c14": {}, "c14u": {"ENOMOTO_MIP_STALL_SCALE": "u"},
        "c14ud": {"ENOMOTO_MIP_STALL_SCALE": "u", "ENOMOTO_MIP_DUALEQ": "1"},
        "c14dg": {"ENOMOTO_MIP_DUALEQ": "1", "ENOMOTO_T_MIP_DUALEQ_MINRED": "0.01"},
        "c14r": {}, "c14x": {"ENOMOTO_MIP_POOL_TAB": "x"}, "c14dx": {"ENOMOTO_MIP_POOL_TAB": "x", "ENOMOTO_MIP_DUALEQ": "1"}}
DQ = {"binkar10_1", "beavma", "flugpl", "neos-960392"}
out = Path(sys.argv[1]); seeds = [int(s) for s in sys.argv[2].split(",")]
given = sys.argv[3].split(",") if len(sys.argv) > 3 else None
probs = sorted({x[0] for s in json.loads((WT / "analysis/cand1114_20261113/shards_P.json").read_text())["shards"] for x in s})
done = set()
if out.exists():
    for l in out.read_text().splitlines():
        if l.strip(): r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))
for seed in seeds:
    for p in probs:
        for cfg in (given or (["c14", "c14u"] + (["c14ud", "c14dg"] if p in DQ else []))):
            if (p, cfg, seed) in done: continue
            mps = rb.fetch(p, WT / ".miplib_cache")
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(CAND1114); env.update(CFGS[cfg]); env["ENOMOTO_MIP_THREADS"] = "4"; env["ENOMOTO_T_MIP_SEED"] = str(seed)
            t0 = time.time()
            try:
                pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                                    cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, timeout=360)
                res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[:200]}
            with open(out, "a") as f:
                f.write(json.dumps({"problem": p, "cfg": cfg, "seed": seed, "tl": 60, "result": res, "wall": round(time.time() - t0, 1), "at": time.strftime("%H:%M:%S")}) + "\n")
            print(p, cfg, seed, res.get("status"), res.get("time"), flush=True)

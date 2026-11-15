"""根の下界を測る (2026-11-07): ENOMOTO の既定 (ENOMOTO_MIP_NODE_LIMIT=1、4 スレッド) の root_bound と、HiGHS (mip_max_nodes=1、4 スレッド) の下界。
使い方: python roots.py 時間 問題... (結果は roots.jsonl に足す。済んだ (問題, 種類) は飛ばす)"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
OUT = D / "roots.jsonl"
tl = float(sys.argv[1])
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["kind"]))
for p in sys.argv[2:]:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for kind in ("ours", "highs"):
        if (p, kind) in done:
            continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_T_MIP_SEED="1")
        if kind == "ours":
            env["ENOMOTO_MIP_NODE_LIMIT"] = "1"
        else:
            env["ENOMOTO_BENCH_HIGHS_OPT"] = "mip_max_nodes=1"
        t0 = time.time()
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), f"--worker-{kind}", str(mps), "--time-limit", str(tl), "--threads", "4"],
                               cwd=WT, env=env, capture_output=True, text=True, timeout=tl * 2 + 120)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            err = r.stderr
        except Exception as e:  # noqa: BLE001
            res, err = {"status": "crash", "error": str(e)[:300]}, ""
        loops = [l for l in err.splitlines() if "cut loop done" in l][:6]
        rec = {"problem": p, "kind": kind, "tl": tl, "wall": round(time.time() - t0, 1), "result": res, "loops": loops}
        with open(OUT, "a") as f:
            f.write(json.dumps(rec) + "\n")
        print(p, kind, res.get("status"), res.get("root_bound"), res.get("bound"), res.get("time"), flush=True)

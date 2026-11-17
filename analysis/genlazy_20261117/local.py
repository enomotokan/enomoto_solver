"""手元の確かめ: python local.py 問題:秒 ... -- 設定 ... (4 スレッド・種 1、ENOMOTO_MIP_LOG=1)。res_local.jsonl に足す (同じ (問題, 設定, 秒) は飛ばす)。
設定は cfg.py の CFGS。GEN_LAZY の診断・数の行と、ノードの LP の数 (node diag) の行を残す。"""
import json, os, re, subprocess, sys, tempfile, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CFGS  # noqa: E402
i = sys.argv.index("--")
PROBS = [(a.split(":")[0], int(a.split(":")[1])) for a in sys.argv[1:i]]
CF = sys.argv[i + 1:]
OUT = D / "res_local.jsonl"
done = {(r["problem"], r["cfg"], r["tl"]) for r in (json.loads(l) for l in OUT.read_text().splitlines() if l.strip())} if OUT.exists() else set()
KEEP = re.compile(r"GENLAZY|GEN_LAZY|node diag|TRSTAT: tree start")
for p, tl in PROBS:
    for c in CF:
        if (p, c, tl) in done:
            continue
        mps = rb.fetch(p, WT / ".miplib_cache")
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1")
        env.update(CFGS[c])
        t0 = time.time()
        with tempfile.TemporaryFile("w+") as err:
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                                cwd=WT, env=env, stdout=subprocess.PIPE, stderr=err, text=True, timeout=tl + 600)
            err.seek(0)
            lines = [l for l in err.read().splitlines() if KEEP.search(l)]
        try:
            res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        except Exception:  # noqa: BLE001
            res = {"status": "crash", "returncode": pr.returncode}
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "tl": tl, "result": res, "lines": lines, "wall": round(time.time() - t0, 1)}) + "\n")
        print(p, c, res.get("status"), round(res.get("time") or 0, 1), res.get("objective"), res.get("bound"), res.get("nodes"), flush=True)

"""4 スレッドで問題を設定ごとに解き、bench_<名前>.jsonl に足す (2026-11-10)。種は ENOMOTO_T_MIP_SEED=回。設定の順は回ごとに逆にする。
使い方: python bench.py 名前 設定,... 回 秒 [問題,...] (問題を省けば 58 問)"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029")); sys.path.insert(0, str(WT / "scripts"))
from probs import PROBS  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "pre": {"ENOMOTO_MIP_SYMPRE": "1"}, "d1": {"ENOMOTO_MIP_SYMPRE": "d1"}, "probe": {"ENOMOTO_MIP_SYMPRE": "probe"}}
name, cfgs, rep, tl = sys.argv[1], sys.argv[2].split(","), int(sys.argv[3]), int(sys.argv[4])
probs = sys.argv[5].split(",") if len(sys.argv) > 5 else PROBS
OUT = D / f"bench_{name}.jsonl"
if rep % 2 == 0:
    cfgs = cfgs[::-1]
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
for p in probs:
    for c in cfgs:
        if (p, c, rep) in done:
            continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_T_MIP_SEED=str(rep), **CFGS[c])
        sym, pre = "", ""
        try:
            mps = rb.fetch(p, WT / ".miplib_cache")
            r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                                "--time-limit", str(tl), "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 300)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            sym = next((l for l in r.stderr.splitlines() if l.startswith("MIP: symmetry: ") and ("generators," in l or "none (" in l)), "")
            pre = " | ".join(l[16:] for l in r.stderr.splitlines() if "SYMPRE" in l)[:300]
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": str(e)[:300]}
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "tl": tl, "result": res, "sym": sym[:200], "sympre": pre, "at": time.strftime("%H:%M:%S")}) + "\n")
        print(p, c, rep, res.get("status"), res.get("time"), res.get("objective"), flush=True)

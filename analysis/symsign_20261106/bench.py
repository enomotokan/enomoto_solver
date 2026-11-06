"""4 スレッドで問題を設定ごとに解き、bench_<名前>.jsonl に足す (2026-11-06)。設定の順は回ごとに逆にする。
使い方: python bench.py 名前 設定,... 回 秒 [問題,...] (問題を省けば 58 問)"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
CFGS = {"def": {}, "zero": {"ENOMOTO_MIP_SYMZERO": "1"}, "sign": {"ENOMOTO_MIP_SYMSIGN": "1"}, "scale": {"ENOMOTO_MIP_SYMSCALE": "1"},
        "both": {"ENOMOTO_MIP_SYMSIGN": "1", "ENOMOTO_MIP_SYMSCALE": "1"}}
name, cfgs, rep, tl = sys.argv[1], sys.argv[2].split(","), int(sys.argv[3]), int(sys.argv[4])
probs = sys.argv[5].split(",") if len(sys.argv) > 5 else PROBS
OUT = D / f"bench_{name}.jsonl"
if rep % 2:
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
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", **CFGS[c])
        sym = ""
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                                "--time-limit", str(tl), "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 300)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            sym = next((l for l in r.stderr.splitlines() if l.startswith("MIP: symmetry: ") and ("generators," in l or "none (" in l)), "")
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": str(e)[:300]}
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "tl": tl, "result": res, "sym": sym[:160], "at": time.strftime("%H:%M:%S")}) + "\n")
        print(p, c, rep, res.get("status"), res.get("time"), flush=True)

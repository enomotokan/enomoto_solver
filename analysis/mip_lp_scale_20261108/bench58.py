"""58 問 (analysis/cbcimpl_20261029/probs.py) を def (2026-11-08 の既定) と h (`ENOMOTO_MIP_LPSCALE=h ENOMOTO_T_MIP_LPSCALE_MIN_RANGE=1e4`) で
4 スレッド・60 秒、種 = 回 (`ENOMOTO_T_MIP_SEED`) で解き、bench58.jsonl に足す。h は `ENOMOTO_MIP_LPSCALE_LOG=1` で切り替えの行と数を notes に残す。
使い方: python bench58.py <種>"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
CFGS = {"def": {}, "h": {"ENOMOTO_MIP_LPSCALE": "h", "ENOMOTO_T_MIP_LPSCALE_MIN_RANGE": "1e4", "ENOMOTO_MIP_LPSCALE_LOG": "1"}}
OUT = D / "bench58.jsonl"
rep = int(sys.argv[1])
cfgs = ["def", "h"] if rep % 2 == 0 else ["h", "def"]
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
base = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
for p in PROBS:
    for c in cfgs:
        if (p, c, rep) in done:
            continue
        env = dict(base, ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_T_MIP_SEED=str(rep), **CFGS[c])
        notes = []
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                                "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=300)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            err = r.stderr.splitlines()
            notes = [l[:160] for l in err if "heavy unscaled" in l or "scaled twin" in l][:3] + [l for l in err if l.startswith("LPSCALE: lps")][-1:]
            notes += [l[:160] for l in err if l.startswith("MIP: root LP ") and "iters" in l][:1]
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": str(e)[:300]}
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "result": res, "notes": notes, "at": time.strftime("%H:%M:%S")}) + "\n")
        print(p, c, rep, res.get("status"), res.get("time"), flush=True)

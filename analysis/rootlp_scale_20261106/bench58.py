"""58 問 (analysis/cbcimpl_20261029/probs.py) を ROOTLP_SCALE を既定にした新しい既定 (def) で 4 スレッド・60 秒で解き、bench58.jsonl に足す (2026-11-07)。
`ENOMOTO_SIMPLEX_SCALE_LOG=1` で尺度の調整が働いたか (`SIMPLEX_SCALE:` の行) を記録に残す。使い方: python bench58.py <回> <設定,...>"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
CFGS = {"def": {}, "prerootscale": {"ENOMOTO_MIP_ROOTLP_SCALE": "0"}}
OUT = D / "bench58.jsonl"
rep = int(sys.argv[1])
cfgs = sys.argv[2].split(",")
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
base = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
KEYS = ("SIMPLEX_SCALE:", "re-solved with the scaled", "root LP failed", "re-solving with the interior", "MIP: root LP ")
for p in PROBS:
    for c in cfgs:
        if (p, c, rep) in done:
            continue
        env = dict(base, ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_SIMPLEX_SCALE_LOG="1", **CFGS[c])
        notes = []
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                                "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=300)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            notes = [l[:200] for l in r.stderr.splitlines() if any(k in l for k in KEYS)][:8]
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": str(e)[:300]}
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "result": res, "notes": notes, "at": time.strftime("%H:%M:%S")}) + "\n")
        print(p, c, rep, res.get("status"), res.get("time"), flush=True)

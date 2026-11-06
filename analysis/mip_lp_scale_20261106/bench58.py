"""58 問 (analysis/cbcimpl_20261029/probs.py) を base・fix で 4 スレッド・60 秒で解き、bench58.jsonl に足す (2026-11-06)。
base = 既定、fix = `ENOMOTO_MIP_LPSCALE=1` (条件なし: 尺度が 1 でない LP はすべて双子を持つ)。fix は `ENOMOTO_MIP_LPSCALE_LOG=1` で
双子を作った LP (行・列・係数の幅) と最後の数の行を notes に残す。
使い方: python bench58.py <回>  (設定の順は回ごとに逆にする。済んだ組は飛ばす)"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
CFGS = {"base": {}, "fix": {"ENOMOTO_MIP_LPSCALE": "1", "ENOMOTO_MIP_LPSCALE_LOG": "1"}}
OUT = D / "bench58.jsonl"
rep = int(sys.argv[1])
cfgs = ["base", "fix"] if rep % 2 == 0 else ["fix", "base"]
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
base = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
for p in PROBS:
    for c in cfgs:
        if (p, c, rep) in done:
            continue
        env = dict(base, ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", **CFGS[c])
        notes = []
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                                "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=300)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            err = r.stderr.splitlines()
            notes = [l[:160] for l in err if "scaled twin" in l][:3] + [l for l in err if l.startswith("LPSCALE: lps")][-1:]
            notes += [l[:160] for l in err if l.startswith("MIP: root LP ") and "iters" in l][:1]
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": str(e)[:300]}
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "result": res, "notes": notes, "at": time.strftime("%H:%M:%S")}) + "\n")
        print(p, c, rep, res.get("status"), res.get("time"), flush=True)

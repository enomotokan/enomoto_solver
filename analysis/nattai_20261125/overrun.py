"""POLISH_STRICT の時間の上限の超過を測る: LP の重い問題を 1 スレッド・時間の上限 TL 秒で、既定・STRICT=1・STRICT=2 で 1 回ずつ解き、
返るまでの時間 (worker の time) と全体の時間 (wall) を overrun.jsonl に足す。使い方: python overrun.py [TL]"""
import json, os, subprocess, sys, time
from pathlib import Path
R = Path(__file__).resolve().parents[2]; D = Path(__file__).resolve().parent
TL = float(sys.argv[1]) if len(sys.argv) > 1 else 60.0
PROBS = ["dano3_3", "dano3_5", "neos-5114902-kasavu", "s100", "savsched1", "rail02", "neos-5104907-jarama", "buildingenergy",
         "neos-5049753-cuanza", "square47"]
CFGS = {"def": {}, "s1": {"ENOMOTO_MIP_POLISH_STRICT": "1"}, "s2": {"ENOMOTO_MIP_POLISH_STRICT": "2", "ENOMOTO_MIP_POLISH_STATS": "1"}}
sys.path.insert(0, str(R / "scripts"))
import run_miplib_benchmark as rb
done = set()
if (D / "overrun.jsonl").exists():
    done = {(r["problem"], r["cfg"]) for r in map(json.loads, open(D / "overrun.jsonl"))}
for p in PROBS:
    mps = rb.fetch(p, R / ".miplib_cache")
    for c, env in CFGS.items():
        if (p, c) in done:
            continue
        e = dict(os.environ); e.update(env)
        t = time.perf_counter()
        try:
            cp = subprocess.run([sys.executable, str(R / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps),
                                 "--time-limit", str(TL), "--threads", "1"], env=e, capture_output=True, text=True, timeout=TL + 1800)
            out, err = cp.stdout, cp.stderr
        except subprocess.TimeoutExpired:
            out, err = "", "timeout"
        wall = time.perf_counter() - t
        res = None
        for l in out.splitlines()[::-1]:
            if l.startswith("{"):
                res = json.loads(l); break
        st = [l for l in err.splitlines() if "POLISH_STRICT=2" in l]
        row = {"problem": p, "cfg": c, "tl": TL, "wall": round(wall, 2), "result": res, "polish": st[-1] if st else None}
        print(json.dumps(row), flush=True)
        with open(D / "overrun.jsonl", "a") as f:
            f.write(json.dumps(row) + "\n")

"""LU_TINY_HS が P で効かなかった理由の切り分け (1 節): P (analysis/cbcimpl_20261029/probs.py の 58 問) を種 1・1 スレッド・60 秒で、
既定 (def) と ENOMOTO_LU_TINY_HS=1i (t1i) で解き、ENOMOTO_MIP_XPROF の単体法の主ループの秒 (`main`) と ENOMOTO_DEBUG_TICKSUM の
LP の反復 (`lpit`) を tinyp.jsonl に書く。途中から再開できる。リポジトリの根から: python analysis/lpiter_20261127/tinyp.py"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
R = D.parents[1]
sys.path.insert(0, str(R / "analysis/cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
OUT = D / "tinyp.jsonl"
CF = {"def": {}, "t1i": {"ENOMOTO_LU_TINY_HS": "1i"}}
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"]))
for i, p in enumerate(PROBS):
    for c in (list(CF) if i % 2 == 0 else list(CF)[::-1]):
        if (p, c) in done: continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(CF[c]); env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_DEBUG_TICKSUM": "1", "ENOMOTO_MIP_XPROF": "1"})
        t0 = time.time()
        try:
            pr = subprocess.run([sys.executable, str(R / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(R / ".miplib_cache" / f"{p}.mps"),
                                 "--time-limit", "60", "--threads", "1"], cwd=R, env=env, capture_output=True, text=True, timeout=400)
            out, err = pr.stdout, pr.stderr
        except subprocess.TimeoutExpired:
            out, err = "", ""
        try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash"}  # noqa: BLE001
        xp = {}
        for l in err.splitlines():
            m = re.match(r"XPROF (\w+)\s+([\d.]+) ms", l)
            if m: xp[m.group(1)] = xp.get(m.group(1), 0.0) + float(m.group(2))
        tk = {}
        ts = [l for l in err.splitlines() if l.startswith("TICKSUM")]
        if ts:
            for w in ts[-1].split()[1:]:
                a, b = w.split("="); tk[a] = float(b)
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "result": res, "xprof": xp, "ticksum": tk, "wall": round(time.time() - t0, 1)}) + "\n")
        print(p, c, res.get("status"), res.get("time"), res.get("nodes"), xp.get("main"), tk.get("lpit"), flush=True)

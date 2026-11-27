"""4 問 (uccase12・bnatt400・fhnw-binpack4-4・rocI-4-11) を MIP 全体で、既定・試作で解き比べる (4 節)。1 スレッド・種 1・300 秒。
ENOMOTO_MIP_XPROF の単体法の主ループの秒 (`main`) と ENOMOTO_DEBUG_TICKSUM の LP の反復 (`lpit`) を mip4.jsonl に書く。途中から再開できる。
使い方 (リポジトリの根から): python analysis/lpiter_20261127/mip4.py [秒]"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
R = D.parents[1]
OUT = D / "mip4.jsonl"
TL = sys.argv[1] if len(sys.argv) > 1 else "300"
PROBS = ["fhnw-binpack4-4", "rocI-4-11", "bnatt400", "uccase12"]
CF = {"def": {}, "rt": {"ENOMOTO_LU_RETA_TICK": "1"}, "t1i": {"ENOMOTO_LU_TINY_HS": "1i"}, "rti": {"ENOMOTO_LU_RETA_TICK": "1", "ENOMOTO_LU_TINY_HS": "1i"}}
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"]))
for p in PROBS:
    for c in CF:
        if (p, c) in done: continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(CF[c]); env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_DEBUG_TICKSUM": "1", "ENOMOTO_MIP_XPROF": "1"})
        t0 = time.time()
        try:
            pr = subprocess.run([sys.executable, str(R / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(R / ".miplib_cache" / f"{p}.mps"),
                                 "--time-limit", TL, "--threads", "1"], cwd=R, env=env, capture_output=True, text=True, timeout=float(TL) * 2 + 300)
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
            f.write(json.dumps({"problem": p, "cfg": c, "tl": TL, "result": res, "xprof": xp, "ticksum": tk, "wall": round(time.time() - t0, 1)}) + "\n")
        print(p, c, res.get("status"), res.get("time"), res.get("nodes"), res.get("root_bound"), xp.get("main"), tk.get("lpit"), tk.get("builds"), flush=True)

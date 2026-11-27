"""LP の時間の割合が大きい問題を 240 問から選ぶための下見 (6 節、親の依頼 2)。combo3 s2 の def_t1 で 30 秒以上かかった (または時間切れの) 問題を、
既定・1 スレッド・種 1・[秒] (既定 60) で解き、ENOMOTO_MIP_XPROF の単体法の主ループの秒 (`main`) と ENOMOTO_DEBUG_TICKSUM の LP の反復 (`lpit`) を
lpshare.jsonl に書く。途中から再開できる。使い方 (リポジトリの根から): python analysis/lpiter_20261127/lpshare.py [秒]"""
import glob, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
R = D.parents[1]
OUT = D / "lpshare.jsonl"
TL = sys.argv[1] if len(sys.argv) > 1 else "60"
S2 = R / "analysis/combo3_20261126/s2"
dt = {}
for f in glob.glob(str(S2 / "res_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "def_t1": dt[r["problem"]] = r["result"]
PROBS = sorted(p for p, v in dt.items() if v.get("status") == "time_limit" or (v.get("time") or 0) >= 30)
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        done.add(json.loads(l)["problem"])
print(len(PROBS), "問", flush=True)
for p in PROBS:
    if p in done: continue
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_DEBUG_TICKSUM": "1", "ENOMOTO_MIP_XPROF": "1"})
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
        f.write(json.dumps({"problem": p, "tl": TL, "result": res, "xprof": xp, "ticksum": tk, "wall": round(time.time() - t0, 1)}) + "\n")
    tm = res.get("time") or 0
    print(p, res.get("status"), round(tm, 1), xp.get("main"), round(xp.get("main", 0) / 1000 / tm, 2) if tm else None, tk.get("lpit"), flush=True)

"""LP の時間の割合が大きい問題 (lpshare.py の下見から選んだもの、lpbig_shards.json) を、既定と試作で 1・4 スレッド・種 1・600 秒で解く (8 節、親の依頼 2)。
python analysis/lpiter_20261127/lpbig.py K → lpbig_sK.jsonl (lpbig_shards.json の K 番目の (問題, 設定) を順に)。止めても同じコマンドで続きから回る。
ENOMOTO_MIP_XPROF の単体法の主ループの秒 (`main`、4 スレッドは全スレッドの和) と ENOMOTO_DEBUG_TICKSUM の LP の反復 (`lpit`) も書く。
ソルバーは同時に 1 つ (4 スレッドの回は 4 コアを使うので、ほかの計測と並べない)。上限 + 300 秒を過ぎたら打ち切って killed とする。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
K = int(sys.argv[1])
mine = json.loads((D / "lpbig_shards.json").read_text())["shards"][K]
OUT = D / f"lpbig_s{K}.jsonl"
TL = 600
PROTO = {"ENOMOTO_LU_RETA_TICK": "1", "ENOMOTO_LU_TINY_HS": "1i", "ENOMOTO_LU_BTRAN_TINY": "1"}
CFGS = {"def_t1": {}, "rtib_t1": PROTO, "def_t4": {}, "rtib_t4": PROTO}
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l)
        if r["result"].get("status") not in ("crash",): done.add((r["problem"], r["cfg"]))
try:
    COMMIT = subprocess.run(["git", "-C", str(WT), "rev-parse", "--short", "HEAD"], capture_output=True, text=True).stdout.strip()
except Exception:  # noqa: BLE001
    COMMIT = None
for p, cfg in mine:
    if (p, cfg) in done: continue
    th = "1" if cfg.endswith("_t1") else "4"
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CFGS[cfg]); env.update({"ENOMOTO_MIP_THREADS": th, "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_DEBUG_TICKSUM": "1", "ENOMOTO_MIP_XPROF": "1"})
    t0 = time.time()
    try:
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                             "--time-limit", str(TL), "--threads", th], cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 300)
        out, err = pr.stdout, pr.stderr
    except subprocess.TimeoutExpired:
        out, err = "", ""
    try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "killed" if not out else "crash"}  # noqa: BLE001
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
        f.write(json.dumps({"problem": p, "cfg": cfg, "seed": 1, "tl": TL, "result": res, "xprof": xp, "ticksum": tk, "wall": round(time.time() - t0, 1),
                            "shard": K, "commit": COMMIT, "at": time.strftime("%m-%d %H:%M:%S")}) + "\n")
    print(p, cfg, res.get("status"), res.get("time"), res.get("nodes"), xp.get("main"), tk.get("lpit"), flush=True)

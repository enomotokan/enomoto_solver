"""基準と試作を同じ機械で交互に回す (4 スレッドの揺れの計測の分担用):
python pair.py スレッド数 分担 分担の数 設定,設定,... [秒=60] [問題,...]
58 問 (既定) か指定の問題 × 変種 VARIANTS の (問題, 並べ替え, 種) を通し番号 % 分担の数 == 分担 だけ受け持ち、各回で設定を順に
(回ごとに順を入れ替えて) 解く。結果は res_<設定>_t<スレッド>_<タグ>.jsonl (タグは環境変数 ROBUST2_TAG、既定は機械の名前)、
ログは logs/。途中から再開できる。4 スレッドは機械を占有して 1 本ずつ回す (CPU 0-3)。
環境変数 ROBUST2_PKG に enomoto_solver の写しの場所を入れると、それで解く (計測中の build の入れ替えを避ける)。"""
import gzip, json, os, socket, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029")); sys.path.insert(0, str(WT / "analysis/cand1114_20261113")); sys.path.insert(0, str(WT / "analysis/robust_20261114")); sys.path.insert(0, str(D))
from probs import PROBS, rb  # noqa: E402
from cfg import CAND1114  # noqa: E402
import perm as pm  # noqa: E402
from cfgs2 import CFGS, VARIANTS  # noqa: E402
TH, SH, NSH = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
cfgs = sys.argv[4].split(",")
TL = float(sys.argv[5]) if len(sys.argv) > 5 else 60.0
probs = sys.argv[6].split(",") if len(sys.argv) > 6 else PROBS
TAG = os.environ.get("ROBUST2_TAG") or socket.gethostname()
LOGS = D / "logs"; LOGS.mkdir(exist_ok=True)
def out_of(c): return D / f"res_{c}_t{TH}_{TAG}.jsonl"
def done_set(c):
    s = set(); f = out_of(c)
    if f.exists():
        for l in f.read_text().splitlines():
            if l.strip(): r = json.loads(l); s.add((r["problem"], r["perm"], r["seed"]))
    return s
def mps_of(p, k):
    src = rb.fetch(p, WT / ".miplib_cache")
    if k == 0: return src
    out = WT / ".miplib_cache/perm" / f"{p}_p{k}.mps"
    if not out.exists(): out.parent.mkdir(exist_ok=True); pm.make(src, k, out)
    return out
items = [(p, k, s) for p in probs for (k, s) in VARIANTS]
mine = [it for i, it in enumerate(items) if i % NSH == SH]
done = {c: done_set(c) for c in cfgs}
CPUS = os.environ.get("ROBUST2_CPUS") or f"0-{os.cpu_count() - 1}"
for n, (p, k, s) in enumerate(mine):
    order = cfgs if n % 2 == 0 else cfgs[::-1]
    for c in order:
        if (p, k, s) in done[c]: continue
        mps = mps_of(p, k)
        env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
        if os.environ.get("ROBUST2_PKG"): env["PYTHONPATH"] = os.environ["ROBUST2_PKG"]
        env.update(CAND1114); env.update(CFGS[c]); env["ENOMOTO_MIP_THREADS"] = str(TH); env["ENOMOTO_T_MIP_SEED"] = str(s); env["ENOMOTO_MIP_LOG"] = "1"
        t0 = time.time()
        with gzip.open(LOGS / f"{p}_{c}_t{TH}_p{k}s{s}.log.gz", "wt") as lf:
            pr = subprocess.Popen(["taskset", "-c", CPUS, sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps),
                                   "--time-limit", str(TL), "--threads", str(TH)], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            out, err = pr.communicate(timeout=TL + 600)
            lf.write(err)
        try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
        row = {"problem": p, "cfg": c, "threads": TH, "perm": k, "seed": s, "tl": TL, "result": res, "wall": round(time.time() - t0, 1),
               "shard": SH, "nshard": NSH, "tag": TAG, "nproc": os.cpu_count(), "at": time.strftime("%m-%d %H:%M:%S")}
        with open(out_of(c), "a") as f: f.write(json.dumps(row) + "\n")
        print(p, c, TH, k, s, res.get("status"), res.get("time"), flush=True)

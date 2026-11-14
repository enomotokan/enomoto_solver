"""揺れの計測 (robust_20261114/run.py に設定と問題の絞り込みを足したもの):
python run2.py 設定 スレッド数 枠 枠の数 [秒=60] [問題,...|@ファイル]
設定は CFGS の名前 (束 cand1114 に足す環境変数)。58 問 (既定) か指定の問題 × 変種 VARIANTS (並べ替え, 種) を解き、
res_<設定>_t<スレッド>.jsonl に足す (途中から再開できる)。枠 i は通し番号 % 枠の数 == i の回を受け持ち、
1 スレッドは CPU i、4 スレッドは CPU 0-3 を使う。ログ (ENOMOTO_MIP_LOG=1) は logs/ に gz で残す。
環境変数 ROBUST2_PKG に enomoto_solver の写しを置いた場所を入れると、それを PYTHONPATH にして解く (計測中の build の入れ替えを避ける)。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029")); sys.path.insert(0, str(WT / "analysis/cand1114_20261113")); sys.path.insert(0, str(WT / "analysis/robust_20261114")); sys.path.insert(0, str(D))
from probs import PROBS, rb  # noqa: E402
from cfg import CAND1114  # noqa: E402
import perm as pm  # noqa: E402
from cfgs2 import CFGS, VARIANTS  # noqa: E402
CFG, TH, SLOT, NSLOT = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
TL = float(sys.argv[5]) if len(sys.argv) > 5 else 60.0
probs = PROBS
if len(sys.argv) > 6:
    a = sys.argv[6]
    probs = (Path(a[1:]).read_text().split() if a.startswith("@") else a.split(","))
OUT = D / f"res_{CFG}_t{TH}.jsonl"
LOGS = D / "logs"; LOGS.mkdir(exist_ok=True)
CPUS = os.environ.get("ROBUST2_CPUS") or (str(SLOT) if TH == 1 else f"0-{os.cpu_count() - 1}")  # ROBUST2_CPUS で CPU を指定できる
def done_set():
    s = set()
    if OUT.exists():
        for l in OUT.read_text().splitlines():
            if l.strip():
                r = json.loads(l); s.add((r["problem"], r["perm"], r["seed"]))
    return s
def mps_of(p, k):
    src = rb.fetch(p, WT / ".miplib_cache")
    if k == 0: return src
    out = WT / ".miplib_cache/perm" / f"{p}_p{k}.mps"
    if not out.exists(): out.parent.mkdir(exist_ok=True); pm.make(src, k, out)
    return out
items = [(p, k, s) for p in probs for (k, s) in VARIANTS]
mine = [it for i, it in enumerate(items) if i % NSLOT == SLOT]
done = done_set()
for p, k, s in mine:
    if (p, k, s) in done: continue
    mps = mps_of(p, k)
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    if os.environ.get("ROBUST2_PKG"): env["PYTHONPATH"] = os.environ["ROBUST2_PKG"]  # 計測中に build し直しても変わらないよう、固定した写しを使う
    env.update(CAND1114); env.update(CFGS[CFG]); env["ENOMOTO_MIP_THREADS"] = str(TH); env["ENOMOTO_T_MIP_SEED"] = str(s); env["ENOMOTO_MIP_LOG"] = "1"
    log = LOGS / f"{p}_{CFG}_t{TH}_p{k}s{s}.log.gz"
    t0 = time.time()
    with gzip.open(log, "wt") as lf:
        pr = subprocess.Popen(["taskset", "-c", CPUS, sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps),
                               "--time-limit", str(TL), "--threads", str(TH)], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        out, err = pr.communicate(timeout=TL + 600)
        lf.write(err)
    try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    row = {"problem": p, "cfg": CFG, "threads": TH, "perm": k, "seed": s, "tl": TL, "result": res, "wall": round(time.time() - t0, 1),
           "slot": SLOT, "nslot": NSLOT, "nproc": os.cpu_count(), "at": time.strftime("%m-%d %H:%M:%S")}
    with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
    print(p, CFG, TH, k, s, res.get("status"), res.get("time"), flush=True)

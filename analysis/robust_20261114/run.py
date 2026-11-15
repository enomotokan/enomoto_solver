"""揺れの計測: python run.py スレッド数 枠 枠の数 [秒=60] [組の名前] [問題,問題,...]
58 問 (analysis/cbcimpl_20261029/probs.py) × 変種 VARIANTS (並べ替え, 種) を束 cand1114 で解き、res_t<スレッド>.jsonl に足す (途中から再開できる)。
枠 i は (問題, 変種) の通し番号 % 枠の数 == i の回を受け持ち、CPU を taskset で分ける (1 スレッドは枠 i が CPU i、4 スレッドは枠 1 つで CPU 0-3)。
組の名前 (cfgs.py) を付けると cand1114 にその環境変数を足し、res_t<スレッド>_<名前>.jsonl・logs/<名前>/ に書く。問題を並べるとそれだけ。
並べ替え 0 は元の MPS、k ≥ 1 は mkperm.py が作った .miplib_cache/perm/<問題>_p<k>.mps。ログ (ENOMOTO_MIP_LOG=1、表示だけ) は logs/ に gz で残す。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029")); sys.path.insert(0, str(WT / "analysis/cand1114_20261113")); sys.path.insert(0, str(D))
from probs import PROBS, rb  # noqa: E402
from cfg import CAND1114  # noqa: E402
import perm as pm  # noqa: E402
from cfgs import CFGS  # noqa: E402
# (並べ替え, 種): 並べ替えだけの揺れ = 種 1 の 4 回、種だけの揺れ = 並べ替え 0 の 3 回、全部 = 6 回
VARIANTS = [(0, 1), (1, 1), (2, 1), (3, 1), (0, 2), (0, 3)]
TH, SLOT, NSLOT = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
TL = float(sys.argv[4]) if len(sys.argv) > 4 else 60.0
NAME = sys.argv[5] if len(sys.argv) > 5 else None
EXTRA = CFGS[NAME] if NAME else {}
ONLY = sys.argv[6].split(",") if len(sys.argv) > 6 else None
OUT = D / (f"res_t{TH}_{NAME}.jsonl" if NAME else f"res_t{TH}.jsonl")
LOGS = D / "logs" / NAME if NAME else D / "logs"; LOGS.mkdir(parents=True, exist_ok=True)
CPUS = str(SLOT) if TH == 1 else f"0-{os.cpu_count() - 1}"
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
    if not out.exists(): pm.make(src, k, out)
    return out
items = [(p, k, s) for p in (ONLY or PROBS) for (k, s) in VARIANTS]
mine = [it for i, it in enumerate(items) if i % NSLOT == SLOT]
done = done_set()
for p, k, s in mine:
    if (p, k, s) in done: continue
    mps = mps_of(p, k)
    env = {key: v for key, v in os.environ.items() if not key.startswith("ENOMOTO_")}
    env.update(CAND1114); env.update(EXTRA); env["ENOMOTO_MIP_THREADS"] = str(TH); env["ENOMOTO_T_MIP_SEED"] = str(s); env["ENOMOTO_MIP_LOG"] = "1"
    log = LOGS / f"{p}_t{TH}_p{k}s{s}.log.gz"
    t0 = time.time()
    with gzip.open(log, "wt") as lf:
        pr = subprocess.Popen(["taskset", "-c", CPUS, sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps),
                               "--time-limit", str(TL), "--threads", str(TH)], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        out, err = pr.communicate(timeout=TL + 600)
        lf.write(err)
    try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    row = {"problem": p, "threads": TH, "perm": k, "seed": s, "tl": TL, "result": res, "wall": round(time.time() - t0, 1),
           "cfg": NAME or "base0", "slot": SLOT, "nslot": NSLOT, "nproc": os.cpu_count(), "at": time.strftime("%m-%d %H:%M:%S")}
    with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
    print(p, TH, k, s, res.get("status"), res.get("time"), flush=True)

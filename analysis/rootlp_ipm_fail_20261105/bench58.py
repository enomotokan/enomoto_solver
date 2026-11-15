"""58 問 (analysis/cbcimpl_20261029/probs.py) を 4 スレッド・60 秒で、既定 (base) と試作 (trial) を問題ごとに交互に 1 回ずつ回す。
使い方: [BENCH_REP=回] [BENCH_PROBS="問題 ..."] python bench58.py 出力.jsonl [設定名 ...]  (すでにある (問題, 設定) は飛ばす)。集計は analysis/cbcimpl_20261029/summ.py で読める形。"""
import json, os, subprocess, sys, time
from pathlib import Path
HERE = Path(__file__).resolve().parent
WT = HERE.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
TRIAL = {"ENOMOTO_T_XO_CLEANUP_HINT": "3", "ENOMOTO_XO_IPM_RETRY_PC": "3", "ENOMOTO_XO_NO_BOUND_STOP_ZERO_COST": "1", "ENOMOTO_IPM_KKT_AUG_RATIO": "4"}
TRIAL5 = dict(TRIAL, ENOMOTO_XO_DUAL_MEGIDDO_RACE_NOLIMIT="1", ENOMOTO_MIP_ADOPT_ROW_SCALE="1", ENOMOTO_XO_DUAL_PUSH_FAST="1", ENOMOTO_XO_MIP_SCALE="1",
              ENOMOTO_T_XO_UNCONVERGED_GIVEUP="0", ENOMOTO_T_IPM_NOIMPROVE="10")
CFGS = {"base": {}, "trial": TRIAL, "hint": {"ENOMOTO_T_XO_CLEANUP_HINT": "3"}, "trial5": TRIAL5}
out = Path(sys.argv[1])
names = sys.argv[2:] or ["base", "trial"]
REP = int(os.environ.get("BENCH_REP", "0"))  # 回の番号 (同じ出力に足す追加の回)
probs = os.environ.get("BENCH_PROBS", "").split() or PROBS
done = set()
if out.exists():
    done = {(json.loads(l)["problem"], json.loads(l)["cfg"]) for l in out.open() if l.strip() and json.loads(l)["rep"] == REP}
for k, p in enumerate(probs):
    for cfg in (names if (k + REP) % 2 == 0 else names[::-1]):
        if (p, cfg) in done:
            continue
        mps = rb.fetch(p, WT / ".miplib_cache")
        env = dict(os.environ, **CFGS[cfg])
        t0 = time.time()
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                               cwd=WT, env=env, capture_output=True, text=True, timeout=300)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": str(e)[:300]}
        with out.open("a") as f:
            f.write(json.dumps({"problem": p, "cfg": cfg, "rep": REP, "wall": time.time() - t0, "result": res}) + "\n")
        print(p, cfg, res.get("status"), res.get("time"), res.get("objective"), flush=True)

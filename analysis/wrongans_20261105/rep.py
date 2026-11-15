"""誤答の再現 (2026-11-05)。問題 1 つを種 (ENOMOTO_T_MIP_SEED) を変えて何回も解き、返した解を元の問題で確かめて 1 回 1 行で足す。
使い方: python rep.py <問題> <設定名> <種の始め> <種の終わり> <出力.jsonl> [時間 既定 60] [スレッド 既定 4] [並列 既定 1]
設定名: def (環境変数なし)・stallrw (ROBUST_STALL=rw)・prr (PROBE_REPRESOLVE=1)、ほか "K=V;K=V" で直接。
ログ (ENOMOTO_MIP_LOG=1) は REP_LOG=1 のとき <出力>.logs/<種>.log に残す。"""
import json, os, subprocess, sys, tempfile, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "mittelmann_ready_20261103"))
import check_sol as cs  # noqa: E402
CF = {"def": {}, "stallrw": {"ENOMOTO_MIP_ROBUST_STALL": "rw"}, "prr": {"ENOMOTO_MIP_PROBE_REPRESOLVE": "1"}}
p, cfg, s0, s1, out = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), Path(sys.argv[5])
tl = sys.argv[6] if len(sys.argv) > 6 else "60"
th = sys.argv[7] if len(sys.argv) > 7 else "4"
par = int(sys.argv[8]) if len(sys.argv) > 8 else 1
envc = CF[cfg] if cfg in CF else dict(kv.split("=", 1) for kv in cfg.split(";") if kv)
mps = WT / ".miplib_cache" / f"{p}.mps"
from enomoto_solver import benchmark_highs as bh  # noqa: E402
_, lp = bh._load_lp(mps)
solu = cs.load_solu()
logd = Path(str(out) + ".logs")


def one(seed):
    with tempfile.TemporaryDirectory() as td:
        xp = os.path.join(td, "x.npy")
        env = dict(os.environ, ENOMOTO_MIP_THREADS=th, CHECK_TL=tl, CHECK_TH=th, ENOMOTO_T_MIP_SEED=str(seed), **envc)
        if os.environ.get("REP_LOG"):
            env["ENOMOTO_MIP_LOG"] = "1"
        r = subprocess.run([sys.executable, str(WT / "analysis/mittelmann_ready_20261103/check_sol.py"), "--worker", str(mps), xp], cwd=WT,
                           env=env, capture_output=True, text=True, timeout=float(tl) * 2 + 120)
        if os.environ.get("REP_LOG"):
            logd.mkdir(exist_ok=True)
            (logd / f"{cfg}_{seed}.log").write_text(r.stderr)
        lines = [l for l in r.stdout.splitlines() if l.startswith("{")]
        res = json.loads(lines[-1]) if lines else {"status": "crash", "stderr": r.stderr[-500:]}
        import numpy as np
        chk = cs.check_x(lp, np.load(xp)) if os.path.exists(xp) else None
        if os.path.exists(xp) and os.environ.get("REP_KEEPX"):
            import shutil; shutil.copy(xp, str(out) + f".{cfg}_{seed}.npy")
    ref = solu.get(p, (None, None))[1]
    rec = {"problem": p, "cfg": cfg, "seed": seed, "result": res, "check": chk, "ref": ref,
           "bad": bool(ref is not None and res.get("status") == "optimal" and abs(res["objective"] - ref) > 1e-6 * max(1, abs(ref)))}
    with open(out, "a") as f:
        f.write(json.dumps(rec) + "\n")
    print(seed, res.get("status"), res.get("objective"), res.get("bound"), round(res.get("time", 0), 1), "BAD" if rec["bad"] else "",
          {k: chk[k] for k in ("row_viol_max", "bound_viol_max", "int_viol_max")} if chk else None, flush=True)


with ThreadPoolExecutor(par) as ex:
    list(ex.map(one, range(s0, s1)))

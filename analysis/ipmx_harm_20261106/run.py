"""ipmx の害の切り分け (2026-11-06): 問題を設定ごとに 4 スレッド・60 秒 (RUN_TL) で解き、ログ (logs/<問題>_<設定>_<回>.log) と
結果 (res.jsonl) を書く。ENOMOTO_MIP_LOG=1・ENOMOTO_DEBUG_RACE=1 を付ける (EXTRA_ENV="A=1,B=2" で足せる)。
使い方: python run.py <設定,...> <問題,...> <回数> [初めの回]"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
H = {"ENOMOTO_T_XO_CLEANUP_HINT": "3"}
R = {"ENOMOTO_XO_IPM_RETRY_PC": "3"}
Z = {"ENOMOTO_XO_NO_BOUND_STOP_ZERO_COST": "1"}
K = {"ENOMOTO_IPM_KKT_AUG_RATIO": "4"}
M = {"ENOMOTO_XO_MIP_CLEANUP_HINT": "1"}
CFGS = {
    "def": {},
    "ipmx": {**H, **R, **Z, **K},
    # 1 つずつ外す
    "ipmx_noH": {**R, **Z, **K},
    "ipmx_noR": {**H, **Z, **K},
    "ipmx_noZ": {**H, **R, **K},
    "ipmx_noK": {**H, **R, **Z},
    # 1 つずつ足す
    "H": H, "R": R, "Z": Z, "K": K,
    # MIP の根の LP だけに絞る版
    "M": M,
    "ipmxm": {**M, **R, **Z, **K},
}
TL = os.environ.get("RUN_TL", "60")
if __name__ == "__main__":
    cfgs, probs, reps = sys.argv[1].split(","), sys.argv[2].split(","), int(sys.argv[3])
    first = int(sys.argv[4]) if len(sys.argv) > 4 else 0
    (D / "logs").mkdir(exist_ok=True)
    extra = dict(kv.split("=", 1) for kv in os.environ.get("EXTRA_ENV", "").split(",") if kv)
    tag = os.environ.get("RUN_TAG", "")
    for rep in range(first, first + reps):
        for p in probs:
            for c in (cfgs if rep % 2 == 0 else cfgs[::-1]):
                out = D / "logs" / f"{p}_{c}{tag}_{rep}.log"
                if out.exists():
                    continue
                env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
                env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_DEBUG_RACE="1", ENOMOTO_MIP_THREADS="4", **CFGS[c], **extra)
                t0 = time.time()
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                                    "--time-limit", TL, "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=int(TL) * 3 + 120)
                wall = time.time() - t0
                out.write_text(r.stderr + "\n" + r.stdout)
                try:
                    res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
                except Exception:  # noqa: BLE001
                    res = {"status": "crash", "rc": r.returncode}
                with open(D / "res.jsonl", "a") as f:
                    f.write(json.dumps({"problem": p, "cfg": c + tag, "rep": rep, "tl": int(TL), "extra": extra, "result": res, "wall": round(wall, 1),
                                        "at": time.strftime("%H:%M:%S")}) + "\n")
                print(p, c + tag, rep, res.get("status"), round(res.get("time") or 0, 1), res.get("nodes"), res.get("root_bound"), flush=True)

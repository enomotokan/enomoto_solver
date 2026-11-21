"""1 スレッドで SCIP だけが解ける問題の切り分け: python run.py [--budget 秒]
JOBS の (問題, 設定, 秒) を順に解き、res.jsonl に足す (同じ (問題, 設定, 秒) の行があれば飛ばす = 続きから)。ソルバーは同時に 1 つ。
設定: t4 = 当方の既定 4 スレッド、t1 = 当方の既定 1 スレッド、scip = SCIP (逐次、randomization/randomseedshift = 種)。種はどれも 1。
当方は ENOMOTO_MIP_LOG=1 で標準エラーを logs/<問題>_<設定>_<秒>.log に、SCIP は ENOMOTO_BENCH_SCIP_LOG=1 でログを同じ名前に残す。
--budget: 始めてからの経過 + 次の組の秒 + 余裕 120 秒が budget を超えるなら、その組を始めずに終わる
(バックグラウンドの 2 時間の打ち切りで長い組を途中で失わないため。もう一度同じコマンドで続きから)。"""
import json, os, subprocess, sys, time, argparse
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402

M = ["mcsched", "CMS750_4", "rail507", "istanbul-no-cutoff", "dano3_5", "tbfp-network", "bnatt400"]
JOBS = [("roll3000", "t4", 60), ("roll3000", "t1", 60), ("roll3000", "scip", 60), ("roll3000", "t1", 240)]
for p in M:
    JOBS += [(p, "t4", 600), (p, "t1", 600), (p, "scip", 600), (p, "t1", 2400)]
# 追加の診断 (既定は変えない、既存の試作スイッチ): t1lm = 1 スレッドで根のヒューリスティクスの終わりに暫定解から Local-MIP を 10 秒
# (`ENOMOTO_MIP_ROOT_LMIP`、`ENOMOTO_T_MIP_ROOT_LMIP_SECS`=10、FRAC は 1 にして 10 秒を効かせる)。4 スレッドの FJ のスレッドの Local-MIP の代わりになるかを見る
EXTRA = {"t1lm": {"ENOMOTO_MIP_ROOT_LMIP": "1", "ENOMOTO_T_MIP_ROOT_LMIP_SECS": "10", "ENOMOTO_T_MIP_ROOT_LMIP_FRAC": "1"}}
JOBS += [("CMS750_4", "t1lm", 600)]
OUT = D / "res.jsonl"
LOGS = D / "logs"

def run(p, cfg, tl, mps):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    th = "4" if cfg == "t4" else "1"
    env.update(EXTRA.get(cfg, {}))
    env["ENOMOTO_MIP_THREADS"] = th; env["ENOMOTO_T_MIP_SEED"] = "1"; env["ENOMOTO_BENCH_SCIP_SEED"] = "1"
    LOGS.mkdir(exist_ok=True)
    log = LOGS / f"{p}_{cfg}_{tl}.log"
    if cfg == "scip": env["ENOMOTO_BENCH_SCIP_LOG"] = "1"
    else: env["ENOMOTO_MIP_LOG"] = "1"
    t0 = time.time()
    with open(log, "w") as lf:
        pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-scip" if cfg == "scip" else "--worker-ours",
                               str(mps), "--time-limit", str(tl), "--threads", th],
                              cwd=WT, env=env, stdout=subprocess.PIPE if cfg != "scip" else lf, stderr=lf, text=True)
        try: out, _ = pr.communicate(timeout=tl + 300)
        except subprocess.TimeoutExpired: pr.kill(); out, _ = pr.communicate()
    if cfg == "scip": out = log.read_text(errors="replace")
    try: res = json.loads([l for l in (out or "").splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    return res, round(time.time() - t0, 1)

def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--budget", type=float, default=1e9); a = ap.parse_args()
    done = set()
    if OUT.exists():
        for l in OUT.read_text().splitlines():
            if l.strip(): r = json.loads(l); done.add((r["problem"], r["cfg"], r["tl"]))
    t_start = time.time()
    for p, cfg, tl in JOBS:
        if (p, cfg, tl) in done: continue
        if time.time() - t_start + tl + 120 > a.budget:
            print("budget: 次の組", p, cfg, tl, "は始めずに終わる", flush=True); return
        mps = rb.fetch(p, WT / ".miplib_cache")
        res, wall = run(p, cfg, tl, mps)
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": cfg, "seed": 1, "tl": tl, "result": res, "wall": wall, "nproc": os.cpu_count(),
                                "at": time.strftime("%m-%d %H:%M:%S")}) + "\n")
        print(p, cfg, tl, res.get("status"), res.get("time"), res.get("objective"), res.get("bound"), flush=True)
    print("ALL DONE", flush=True)

if __name__ == "__main__":
    main()

"""群 4 (解探し) の切り分け: python run.py 問題,... [種] [ours|scip|both]
当方 (4 スレッド、ENOMOTO_MIP_LOG=1) と SCIP 10.0.2 (逐次、ログあり) を 1 問ずつ順に解き (同時に 1 つ)、
ログを logs/<who><TAG>_<問題>_s<種>.log.gz に、結果の 1 行を res.jsonl に足す (analysis/heurgap_20261117/run.py の写しを SCIP 向けに直したもの)。
環境変数: TAG (札)、OURS_ENV ("名=値;名=値"、当方に足す環境変数)、SCIP_SET ("パラメータ=値;..."、SCIP に足す設定)、TL (秒、既定 600)。
ログの各行の頭に起動からの実時間 [秒] を付ける (当方のログの時刻は再スタートごとに 0 に戻るため)。"""
import gzip, json, os, subprocess, sys, threading, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[2]
TAG = os.environ.get("TAG", ""); TL = os.environ.get("TL", "600")
XENV = dict(kv.split("=", 1) for kv in os.environ.get("OURS_ENV", "").split(";") if kv)
SSET = dict(kv.split("=", 1) for kv in os.environ.get("SCIP_SET", "").split(";") if kv)
probs = sys.argv[1].split(","); seed = int(sys.argv[2]) if len(sys.argv) > 2 else 1; which = sys.argv[3] if len(sys.argv) > 3 else "both"
SCODE = """
import json, sys, time, pyscipopt
m = pyscipopt.Model()
m.setParam("limits/time", float(sys.argv[2])); m.setParam("limits/gap", 0.0)
if int(sys.argv[3]) > 1: m.setParam("randomization/randomseedshift", int(sys.argv[3]))
for kv in [x for x in sys.argv[4].split(";") if x]:
    k, v = kv.split("=", 1)
    try: v = int(v)
    except ValueError:
        try: v = float(v)
        except ValueError: v = {"true": True, "false": False}.get(v, v)
    m.setParam(k, v)
m.readProblem(sys.argv[1]); t = time.perf_counter(); m.optimize(); t = time.perf_counter() - t
has = m.getNSols() > 0
print("RES " + json.dumps({"status": str(m.getStatus()), "objective": m.getObjVal() if has else None, "bound": m.getDualbound(), "nodes": m.getNTotalNodes(), "time": t}), flush=True)
m.printStatistics()
"""
def stamp(pr, lf, t0, out):
    with gzip.open(lf, "wt") as f:
        for line in pr.stdout:
            f.write(f"[{time.time() - t0:8.2f}] {line}")
            if line.startswith("RES ") or line.startswith("{"): out.append(line)
for p in probs:
    mps = WT / ".miplib_cache" / f"{p}.mps"
    for who in (["ours", "scip"] if which == "both" else [which]):
        lf = D / "logs" / f"{who}{TAG}_{p}_s{seed}.log.gz"
        t0 = time.time(); out = []
        if who == "scip":
            pr = subprocess.Popen([sys.executable, "-c", SCODE, str(mps), TL, str(seed), os.environ.get("SCIP_SET", "")],
                                  stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
        else:
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": str(seed), "ENOMOTO_MIP_LOG": "1"}); env.update(XENV)
            pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", TL, "--threads", "4"],
                                  cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
        stamp(pr, lf, t0, out); pr.wait()
        try:
            l = out[-1]; r = json.loads(l[4:] if l.startswith("RES ") else l)
        except Exception: r = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
        row = {"problem": p, "who": who + TAG, "env": XENV if who == "ours" else SSET, "tl": TL, "seed": seed, "at": time.strftime("%H:%M:%S")} | \
              {k: r.get(k) for k in ("status", "objective", "bound", "nodes", "time", "first_solution_time", "first_solution_objective")}
        with open(D / "res.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
        print(row, flush=True)

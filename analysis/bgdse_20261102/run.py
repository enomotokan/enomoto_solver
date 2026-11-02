"""bgdse (2026-11-02): 裏のカットの写しに DSE 重みを渡す試作 (`ENOMOTO_MIP_BG_CUTS_DSE`) を問題・設定ごとに n 回ずつ解く
(設定を交互に、ソルバーは同時に 1 つ)。4 スレッド・60 秒、ENOMOTO_MIP_LOG=1・ENOMOTO_MIP_BG_CUTS_LOG=1。
使い方: python run.py 問題,... 設定,... 回数 [初めの回]   ログ logs/<問題>_<設定>_<回>.log、結果 res.jsonl
(analysis/bgx_check1_20261102/run.py をもとに)"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {
    "def": {},
    "dse1": {"ENOMOTO_MIP_BG_CUTS_DSE": "1"},
    "dsex": {"ENOMOTO_MIP_BG_CUTS_DSE": "x"},
    "cap1": {"ENOMOTO_MIP_BG_CUTS_LP_CAP": "1"},
    "dse1_s06": {"ENOMOTO_MIP_BG_CUTS_DSE": "1", "ENOMOTO_T_MIP_BG_CUTS_LP_CAP_SECS": "0.6"},
    "def_s06": {"ENOMOTO_T_MIP_BG_CUTS_LP_CAP_SECS": "0.6"},
}
for k, v in json.loads(os.environ.get("BGX_EXTRA_CFGS", "{}")).items():
    CFGS[k] = v
probs = sys.argv[1].split(",")
cfgs = sys.argv[2].split(",")
n = int(sys.argv[3]); i0 = int(sys.argv[4]) if len(sys.argv) > 4 else 0
for i in range(i0, i0 + n):
    for P in probs:
        mps = rb.fetch(P, WT / ".miplib_cache")
        for c in cfgs:
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_BG_CUTS_LOG="1", **CFGS[c])
            r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                               cwd=WT, env=env, capture_output=True, text=True, timeout=400)
            (D / "logs" / f"{P}_{c}_{i}.log").write_text(r.stderr + r.stdout)
            try:
                res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            except IndexError:
                res = {"status": "crash", "time": None, "rc": r.returncode}
            res.update(prob=P, cfg=c, run=i)
            with open(D / "res.jsonl", "a") as f:
                f.write(json.dumps(res) + "\n")
            print(P, c, i, res["status"], res["time"] and round(res["time"], 1), res.get("objective"), flush=True)

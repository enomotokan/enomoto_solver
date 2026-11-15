"""bg_lp_work (2026-11-02): 裏のカットの写しのカットの LP の上限を反復で決める試作 (`ENOMOTO_MIP_BG_CUTS_LP_WORK`) と、
厳密な DSE 重みを列の値を渡す問題だけに絞って止めの合図を見る試作 (`ENOMOTO_MIP_BG_CUTS_DSE=rw`) を、問題・設定ごとに n 回ずつ解く
(設定を交互に、ソルバーは同時に 1 つ)。4 スレッド・60 秒、ENOMOTO_MIP_LOG=1・ENOMOTO_MIP_BG_CUTS_LOG=1。
使い方: python run.py 問題,... 設定,... 回数 [初めの回]   ログ logs/<問題>_<設定>_<回>.log、結果 res.jsonl
(analysis/bgdse_20261102/run.py をもとに)"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
S06 = {"ENOMOTO_T_MIP_BG_CUTS_LP_CAP_SECS": "0.6"}
CFGS = {
    "def": {},
    "w1": {"ENOMOTO_MIP_BG_CUTS_LP_WORK": "1"},
    "w1h": {"ENOMOTO_MIP_BG_CUTS_LP_WORK": "1h"},
    "rw": {"ENOMOTO_MIP_BG_CUTS_DSE": "rw"},
    "w1h_rw": {"ENOMOTO_MIP_BG_CUTS_LP_WORK": "1h", "ENOMOTO_MIP_BG_CUTS_DSE": "rw"},
    "def_s06": S06,
    "w1_s06": {"ENOMOTO_MIP_BG_CUTS_LP_WORK": "1", **S06},
    "w1h_s06": {"ENOMOTO_MIP_BG_CUTS_LP_WORK": "1h", **S06},
    "r": {"ENOMOTO_MIP_BG_CUTS_DSE": "r"},
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

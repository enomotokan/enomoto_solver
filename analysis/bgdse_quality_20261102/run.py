"""bgdse_quality (2026-11-02): 裏のカットの写しの最初のカットの LP の前に、DSE 重みのずれを標本の行で測って厳密な重みを
かけるか決める試作 (`ENOMOTO_MIP_BG_CUTS_DSE=q`・`=1q`) を、既定・`LP_WORK=1`・`+rw` と並べて問題・設定ごとに n 回ずつ解く
(設定を交互に、ソルバーは同時に 1 つ)。4 スレッド・60 秒、ENOMOTO_MIP_LOG=1・ENOMOTO_MIP_BG_CUTS_LOG=1。
使い方: python run.py 問題,... 設定,... 回数 [初めの回]   ログ logs/<問題>_<設定>_<回>.log、結果 res.jsonl
(analysis/bg_lp_work_20261102/run.py をもとに)"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
S06 = {"ENOMOTO_T_MIP_BG_CUTS_LP_CAP_SECS": "0.6"}
W1 = {"ENOMOTO_MIP_BG_CUTS_LP_WORK": "1"}
CFGS = {
    "def": {},
    "w1": W1,
    "w1_rw": {**W1, "ENOMOTO_MIP_BG_CUTS_DSE": "rw"},
    "w1_q": {**W1, "ENOMOTO_MIP_BG_CUTS_DSE": "q"},
    "w1_1q": {**W1, "ENOMOTO_MIP_BG_CUTS_DSE": "1q"},
    "w1_qa": {**W1, "ENOMOTO_MIP_BG_CUTS_DSE": "qa"},
    "w1_1qa": {**W1, "ENOMOTO_MIP_BG_CUTS_DSE": "1qa"},
    # 閾値を決めるための両端 (標本のずれは測るが、常に厳密・決して厳密にしない)
    # (計測 1・2 は判定の統計が今と違う: 計測 1 は全体の |ln| の中央値・閾値 0 / 1e9、計測 2 は主実行不能な行のばらつき・閾値 1e9。どちらも両端なので判定は同じ)
    "w1_q0": {**W1, "ENOMOTO_MIP_BG_CUTS_DSE": "q", "ENOMOTO_T_MIP_BG_CUTS_DSE_DEV": "0"},
    "w1_qinf": {**W1, "ENOMOTO_MIP_BG_CUTS_DSE": "q", "ENOMOTO_T_MIP_BG_CUTS_DSE_DEV": "1e9"},
    "w1_qainf": {**W1, "ENOMOTO_MIP_BG_CUTS_DSE": "qa", "ENOMOTO_T_MIP_BG_CUTS_DSE_DEV": "1e9"},
    "w1_1qinf": {**W1, "ENOMOTO_MIP_BG_CUTS_DSE": "1q", "ENOMOTO_T_MIP_BG_CUTS_DSE_DEV": "1e9"},
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

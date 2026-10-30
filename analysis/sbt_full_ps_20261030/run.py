"""sbt_full_ps の切り分けの計測。設定を交互に回し、結果を jsonl に、ログを logs/ に残す。
使い方: python run.py 出力.jsonl 回数 問題,... 設定,...  (4 スレッド・60 秒、1 本ずつ)"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
S = {"ENOMOTO_MIP_SB_TIGHTEN": "1", "ENOMOTO_MIP_SB_TIGHTEN_INHERIT": "1", "ENOMOTO_MIP_SB_TIGHTEN_ANCESTOR": "1"}
CFGS = {
    "base": {},
    "sbt_abpr": {**S, "ENOMOTO_MIP_SB_POLY_RC": "1"},
    "sbt_full_ps": {**S, "ENOMOTO_MIP_SB_POLY_RC": "1", "ENOMOTO_MIP_SB_RC": "1", "ENOMOTO_MIP_SB_POLY_SCORE": "1"},
    "sbt_abpr_ps": {**S, "ENOMOTO_MIP_SB_POLY_RC": "1", "ENOMOTO_MIP_SB_POLY_SCORE": "1"},
    # sbt_full_ps から 1 つずつ外したもの
    "full_no_ps": {**S, "ENOMOTO_MIP_SB_POLY_RC": "1", "ENOMOTO_MIP_SB_RC": "1"},
    "full_no_rc": {**S, "ENOMOTO_MIP_SB_POLY_RC": "1", "ENOMOTO_MIP_SB_POLY_SCORE": "1"},
    "full_no_prc": {**S, "ENOMOTO_MIP_SB_RC": "1", "ENOMOTO_MIP_SB_POLY_SCORE": "1"},
}
EXTRA = json.loads(os.environ.get("RUN_EXTRA_CFGS", "{}"))
CFGS.update(EXTRA)

out, reps, probs, cfgs = sys.argv[1], int(sys.argv[2]), sys.argv[3].split(","), sys.argv[4].split(",")
rep0 = int(os.environ.get("RUN_REP0", "0"))
(HERE / "logs").mkdir(exist_ok=True)
for rep in range(rep0, rep0 + reps):
    for prob in probs:
        for cfg in cfgs:
            env = dict(os.environ, ENOMOTO_MIP_LOG="1", PYTHONPATH=str(ROOT / "python"), **CFGS[cfg])
            t0 = time.time()
            r = subprocess.run([sys.executable, str(ROOT / "scripts/run_miplib_benchmark.py"), "--worker-ours",
                                str(ROOT / f".miplib_cache/{prob}.mps"), "--time-limit", "60", "--threads", "4"],
                               env=env, capture_output=True, text=True, cwd=ROOT)
            wall = time.time() - t0
            with gzip.open(HERE / "logs" / f"{prob}_{cfg}_{rep}.log.gz", "wt") as g:
                g.write(r.stderr)
            try:
                res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            except Exception:
                res = {"status": "error", "stdout": r.stdout[-500:]}
            with open(out, "a") as f:
                f.write(json.dumps({"problem": prob, "cfg": cfg, "rep": rep, "wall": wall, "result": res}) + "\n")
Path("/tmp/DONE").touch()

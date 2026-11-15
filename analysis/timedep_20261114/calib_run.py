"""手間の上限の当てはめ用のログを取る (analysis/timedep_20261114.md の 10 節): python calib_run.py 問題,... 種,... 設定,...
設定: c = CAND1115C (時間の規則) + HALT、td = CAND1115 (Et2) + HALT。4 スレッド・60 秒、ENOMOTO_MIP_LOG=1、ログは logs_calib/<設定>/<問題>_s<種>.log。"""
import os, subprocess, sys, importlib.util
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
sp = importlib.util.spec_from_file_location("cfg1115", WT / "analysis/cand1115_20261115/cfg.py"); m = importlib.util.module_from_spec(sp); sp.loader.exec_module(m)
H = {"ENOMOTO_MIP_SUBMIP_HALT": "1"}
CFG = {"c": {**m.CAND1115C, **H}, "td": {**m.CAND1115, **H}, "tdf": {**m.CAND1115, **H, "ENOMOTO_T_MIP_TIMEDEP_LNS_FLOOR_WORK": "3e7"}}
probs, seeds, cfgs = sys.argv[1].split(","), sys.argv[2].split(","), sys.argv[3].split(",")
for s in seeds:
    for p in probs:
        mps = rb.fetch(p, WT / ".miplib_cache")
        for c in cfgs:
            L = D / "logs_calib" / c; L.mkdir(parents=True, exist_ok=True)
            lf = L / f"{p}_s{s}.log"
            if lf.exists() and "TRSTAT" in lf.read_text(errors="ignore"): continue
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(CFG[c]); env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": s, "ENOMOTO_MIP_LOG": "1"})
            with open(lf, "w") as f:
                pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=f, text=True)
            print(p, s, c, [l for l in pr.stdout.splitlines() if l.startswith("{")][-1][:160], flush=True)

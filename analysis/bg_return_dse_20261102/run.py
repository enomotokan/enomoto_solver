"""bg_return_dse (2026-11-02): 裏のカットの結果を主が使う問題を既定 (def) と ENOMOTO_MIP_BG_RETURN_DSE=1 (rd) で n 回ずつ解く
(設定を交互に、ソルバーは同時に 1 つ)。4 スレッド・60 秒、ENOMOTO_MIP_LOG=1・ENOMOTO_MIP_BG_CUTS_LOG=1。
使い方: python run.py 設定,... 回数 [初めの回] [問題,...]   ログ logs/<問題>_<設定>_<回>.log、結果 res.jsonl"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "rd": {"ENOMOTO_MIP_BG_RETURN_DSE": "1"}}
PS = ["physiciansched6-2", "mzzv42z", "atlanta-ip", "neos-662469", "neos-957323", "seymour1", "mc11"]
cfgs = sys.argv[1].split(",")
n = int(sys.argv[2]); i0 = int(sys.argv[3]) if len(sys.argv) > 3 else 0
if len(sys.argv) > 4:
    PS = sys.argv[4].split(",")
for i in range(i0, i0 + n):
    for p in PS:
        mps = rb.fetch(p, WT / ".miplib_cache")
        for c in cfgs:
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_BG_CUTS_LOG="1", **CFGS[c])
            r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                               cwd=WT, env=env, capture_output=True, text=True, timeout=300)
            (D / "logs" / f"{p}_{c}_{i}.log").write_text(r.stderr + r.stdout)
            res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
            res.update(problem=p, cfg=c, run=i)
            with open(D / "res.jsonl", "a") as f:
                f.write(json.dumps(res) + "\n")
            print(p, c, i, res["status"], round(res["time"], 1), res.get("objective"), flush=True)

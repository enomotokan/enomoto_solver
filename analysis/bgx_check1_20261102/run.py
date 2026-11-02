"""bgx_check1 (2026-11-02): physiciansched6-2 を設定ごとに n 回ずつ解く (設定を交互に、ソルバーは同時に 1 つ)。
4 スレッド・60 秒、ENOMOTO_MIP_LOG=1・ENOMOTO_MIP_BG_CUTS_LOG=1 (写しの求解のログ、表示だけ)。
使い方: python run.py 設定,... 回数 [初めの回]   ログ logs/<設定>_<回>.log、結果 res.jsonl"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {
    "def": {},
    "x0": {"ENOMOTO_MIP_BG_CUTS_BASIS_X": "0"},
}
for k, v in json.loads(os.environ.get("BGX_EXTRA_CFGS", "{}")).items():
    CFGS[k] = v
P = "physiciansched6-2"
mps = rb.fetch(P, WT / ".miplib_cache")
cfgs = sys.argv[1].split(",")
n = int(sys.argv[2]); i0 = int(sys.argv[3]) if len(sys.argv) > 3 else 0
for i in range(i0, i0 + n):
    for c in cfgs:
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_BG_CUTS_LOG="1", **CFGS[c])
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                           cwd=WT, env=env, capture_output=True, text=True, timeout=300)
        (D / "logs" / f"{c}_{i}.log").write_text(r.stderr + r.stdout)
        res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        res.update(cfg=c, run=i)
        with open(D / "res.jsonl", "a") as f:
            f.write(json.dumps(res) + "\n")
        print(c, i, res["status"], round(res["time"], 1), res.get("objective"), flush=True)

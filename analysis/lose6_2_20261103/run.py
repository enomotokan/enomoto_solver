"""lose6_2 (2026-11-03): HiGHS に負けている physiciansched6-2・hypothyroid-k1 を設定ごとに n 回ずつ解く (設定を交互に、ソルバーは同時に 1 つ)。
4 スレッド・60 秒、ENOMOTO_MIP_LOG=1・ENOMOTO_MIP_BG_CUTS_LOG=1・ENOMOTO_DEBUG_RACE=1 (表示だけ)。設定 `highs` は --worker-highs で時間を、
highs_log.py でログ (logs/<問題>_highs_<回>.log・.trace) を取る (別々の 2 回の実行)。
使い方: python run.py 問題,... 設定,... 回数 [初めの回]   ログ logs/<問題>_<設定>_<回>.log、結果 res.jsonl
追加の設定は環境変数 L62_EXTRA_CFGS に JSON で。"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}}
for k, v in {**json.loads((D / "cfgs.json").read_text()), **json.loads(os.environ.get("L62_EXTRA_CFGS", "{}"))}.items():
    CFGS[k] = v
probs = sys.argv[1].split(",")
cfgs = sys.argv[2].split(",")
n = int(sys.argv[3]); i0 = int(sys.argv[4]) if len(sys.argv) > 4 else 0
(D / "logs").mkdir(exist_ok=True)
for P in probs:
    mps = rb.fetch(P, WT / ".miplib_cache")
    for i in range(i0, i0 + n):
        for c in cfgs:
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            if c == "highs":
                r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-highs", str(mps), "--time-limit", "60", "--threads", "4"],
                                   cwd=WT, env=env, capture_output=True, text=True, timeout=300)
                js = [l for l in r.stdout.splitlines() if l.startswith("{")]
                res = json.loads(js[-1]) if js else {"status": "crash", "time": 300.0}
                r2 = subprocess.run([sys.executable, str(D / "highs_log.py"), str(mps), "60", str(D / "logs" / f"{P}_highs_{i}")],
                                    env=env, capture_output=True, text=True, timeout=300)
                js2 = [l for l in r2.stdout.splitlines() if l.startswith("{")]
                res["logged_run"] = json.loads(js2[-1]) if js2 else None
            else:
                env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_BG_CUTS_LOG="1", ENOMOTO_DEBUG_RACE="1", **CFGS[c])
                try:
                    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                                       cwd=WT, env=env, capture_output=True, text=True, timeout=300)
                    out, err = r.stdout, r.stderr
                except subprocess.TimeoutExpired as e:
                    out, err = "", (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or "")
                (D / "logs" / f"{P}_{c}_{i}.log").write_text(err + out)
                js = [l for l in out.splitlines() if l.startswith("{")]
                res = json.loads(js[-1]) if js else {"status": "crash", "time": 300.0}
            res.update(problem=P, cfg=c, run=i)
            with open(D / "res.jsonl", "a") as f:
                f.write(json.dumps(res) + "\n")
            print(P, c, i, res["status"], round(res["time"], 1), res.get("objective"), flush=True)

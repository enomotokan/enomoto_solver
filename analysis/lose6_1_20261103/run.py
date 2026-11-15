"""lose6_1 (2026-11-03): HiGHS に負けた 6 問のうち h80x6320d・neos-2978193-inde を調べる。問題を設定ごとに n 回ずつ解く
(設定を交互に、ソルバーは同時に 1 つ)。4 スレッド・60 秒。ENOMOTO は ENOMOTO_MIP_LOG=1・ENOMOTO_MIP_BG_CUTS_LOG=1・ENOMOTO_DEBUG_RACE=1
(表示だけ)、HiGHS (設定 highs) は ENOMOTO_BENCH_HIGHS_LOG=1 でログを出す。
使い方: python run.py 問題,... 設定,... 回数 [初めの回]   ログ logs/<問題>_<設定>_<回>.log、結果 res.jsonl
設定の追加は環境変数 L6_EXTRA_CFGS='{"名前": {"ENOMOTO_...": "..."}}'。"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "highs": {}}
for k, v in {**json.loads((D / "cfgs.json").read_text()), **json.loads(os.environ.get("L6_EXTRA_CFGS", "{}"))}.items():
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
                kind = "--worker-highs"; env.update(ENOMOTO_BENCH_HIGHS_LOG="1")
            else:
                kind = "--worker-ours"
                env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_BG_CUTS_LOG="1", ENOMOTO_DEBUG_RACE="1", **CFGS[c])
            try:
                r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), kind, str(mps), "--time-limit", "60", "--threads", "4"],
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

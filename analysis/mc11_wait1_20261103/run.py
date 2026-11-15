"""mc11_wait1 (2026-11-03): mc11 の canddet での遅れの切り分け。問題を設定ごとに n 回ずつ解く (設定を交互に、ソルバーは同時に 1 つ)。
4 スレッド・60 秒、ENOMOTO_MIP_LOG=1・ENOMOTO_MIP_BG_CUTS_LOG=1 (表示だけ)。
使い方: python run.py 問題,... 設定,... 回数 [初めの回]   ログ logs/<問題>_<設定><札>_<回>.log、結果 res.jsonl (札は W1_TAG)"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
# canddet の 5 つと p (どれも =0 で切れる)
OFF = {
    "pbf": "ENOMOTO_MIP_POOL_BASIS_FIX",
    "pdse": "ENOMOTO_MIP_POOL_DSE",
    "fj": "ENOMOTO_MIP_SMALL2_FJSTOP",
    "lw": "ENOMOTO_MIP_BG_CUTS_LP_WORK",
    "det": "ENOMOTO_MIP_BG_CUTS_DETERM",
    "p": "ENOMOTO_MIP_CUT_LP_CAP_ROBUST",
}
CFGS = {"def": {}}
for k, v in OFF.items():
    CFGS["n_" + k] = {v: "0"}
CFGS["n_all"] = {v: "0" for v in OFF.values()}
CFGS["n_cand"] = {v: "0" for k, v in OFF.items() if k != "p"}
# 試作 ENOMOTO_MIP_MC11FIX1_RACE (小さい LP の根の LP の同時実行の勝ち負けを時間で決めない)
CFGS["f1s"] = {"ENOMOTO_MIP_MC11FIX1_RACE": "s"}
CFGS["f1i"] = {"ENOMOTO_MIP_MC11FIX1_RACE": "i"}
# 停滞の判定 (もう 1 つのセッションの範囲) との関係を見るだけの形: f1s + 窓の判定を 1 ラウンド遅らせる (robust_stall の `l`)
CFGS["f1s_l"] = {"ENOMOTO_MIP_MC11FIX1_RACE": "s", "ENOMOTO_MIP_BG_DETERM_ROBUST": "l"}
CFGS["def_l"] = {"ENOMOTO_MIP_BG_DETERM_ROBUST": "l"}
for k, v in json.loads(os.environ.get("W1_EXTRA_CFGS", "{}")).items():
    CFGS[k] = v
if __name__ == "__main__":
    probs = sys.argv[1].split(",")
    cfgs = sys.argv[2].split(",")
    n = int(sys.argv[3]); i0 = int(sys.argv[4]) if len(sys.argv) > 4 else 0
    (D / "logs").mkdir(exist_ok=True)
    TAG = os.environ.get("W1_TAG", "")
    for P in probs:
        mps = rb.fetch(P, WT / ".miplib_cache")
        for i in range(i0, i0 + n):
            for c in cfgs:
                env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
                env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_BG_CUTS_LOG="1", **CFGS[c])
                try:
                    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                                       cwd=WT, env=env, capture_output=True, text=True, timeout=300)
                    out, err = r.stdout, r.stderr
                except subprocess.TimeoutExpired as e:
                    out, err = "", (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or "")
                (D / "logs" / f"{P}_{c}{TAG}_{i}.log").write_text(err + out)
                js = [l for l in out.splitlines() if l.startswith("{")]
                res = json.loads(js[-1]) if js else {"status": "crash", "time": 300.0}
                res.update(problem=P, cfg=c + TAG, run=i)
                with open(D / "res.jsonl", "a") as f:
                    f.write(json.dumps(res) + "\n")
                print(P, c, i, res["status"], round(res["time"], 1), res.get("objective"), flush=True)

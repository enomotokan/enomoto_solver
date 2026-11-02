"""bg_determ (2026-11-02): 問題を設定ごとに n 回ずつ解く (設定を交互に、ソルバーは同時に 1 つ)。
4 スレッド・60 秒、ENOMOTO_MIP_LOG=1・ENOMOTO_MIP_BG_CUTS_LOG=1 (表示だけ)。
使い方: python run.py 問題,... 設定,... 回数 [初めの回]   ログ logs/<問題>_<設定>_<回>.log、結果 res.jsonl"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
S = {"ENOMOTO_MIP_SUBMIP_HIGHS": "s"}
LW = {"ENOMOTO_MIP_BG_CUTS_LP_WORK": "1h"}
CFGS = {
    "def": {},
    "det": {"ENOMOTO_MIP_BG_CUTS_DETERM": "1"},
    "detc": {"ENOMOTO_MIP_BG_CUTS_DETERM": "c"},
    "detrc": {"ENOMOTO_MIP_BG_CUTS_DETERM": "rc"},
    "def_s": S,
    "detc_s": {"ENOMOTO_MIP_BG_CUTS_DETERM": "c", **S},
    "detrc_s": {"ENOMOTO_MIP_BG_CUTS_DETERM": "rc", **S},
    "lw": LW,
    "def_m1": {"ENOMOTO_T_MIP_BG_CUTS_WAIT_MULT": "1"},
    "def_s_m1": {"ENOMOTO_T_MIP_BG_CUTS_WAIT_MULT": "1", **S},
    "detc_s_m1": {"ENOMOTO_MIP_BG_CUTS_DETERM": "c", "ENOMOTO_T_MIP_BG_CUTS_WAIT_MULT": "1", **S},
    "detc_lw": {"ENOMOTO_MIP_BG_CUTS_DETERM": "c", **LW},
}
for k, v in json.loads(os.environ.get("BGD_EXTRA_CFGS", "{}")).items():
    CFGS[k] = v
probs = sys.argv[1].split(",")
cfgs = sys.argv[2].split(",")
n = int(sys.argv[3]); i0 = int(sys.argv[4]) if len(sys.argv) > 4 else 0
(D / "logs").mkdir(exist_ok=True)
TAG = os.environ.get("BGD_TAG", "")  # 版の札 (ログと結果の設定名の後ろに付ける。v1 = 窓の判定の保留なし、札なし = v1)
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

"""裏のカットの下界が刈り込みの上限に届いたら根のヒューリスティクスを止める試作 (`ENOMOTO_MIP_BG_CUTS_PROOF_STOP`) の計測 (2026-10-31、4 スレッド・60 秒)。
`restart_letters_20261031/run.py` と同じ走らせ方。
使い方: python run.py <設定名,...> <問題,...> <回数> [初めの回]。ログは logs/<問題>_<設定>_<回>.log、結果は res.jsonl に 1 行ずつ足す。"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
HS = {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s"}
PI0 = {"ENOMOTO_MIP_SUBMIP_PC_INHERIT": "1", "ENOMOTO_T_MIP_SUBMIP_RELIABLE": "0"}
BGW = {"ENOMOTO_MIP_BG_CUTS_WAIT": "1"}
P = {**HS, **PI0, **BGW}
CGH = {"ENOMOTO_MIP_SUBMIP_HIGHS": "cgh", **BGW}
R = "ENOMOTO_MIP_SUBMIP_HIGHS_RESTART"
CFGS = {
    "def": {},
    "hs_pi0_bgw": P,
    "P_r": {**P, "ENOMOTO_MIP_SUBMIP_HIGHS_TRIM": "r"},
    "cgh_bgw": CGH,
}
# `_ps` は `ENOMOTO_MIP_BG_CUTS_PROOF_STOP=1` を足した形
for k in list(CFGS):
    CFGS[f"{k}_ps"] = {**CFGS[k], "ENOMOTO_MIP_BG_CUTS_PROOF_STOP": "1"}
# 追加の設定は "名前=変数=値;変数=値" の形でも与えられる
def cfg(name):
    if name in CFGS:
        return name, CFGS[name]
    n, rest = name.split("=", 1)
    return n, dict(kv.split("=", 1) for kv in rest.split(";"))
if __name__ == "__main__":
    cfgs = [cfg(c) for c in sys.argv[1].split(",")]
    probs, reps = sys.argv[2].split(","), int(sys.argv[3])
    rep0 = int(sys.argv[4]) if len(sys.argv) > 4 else 0
    (D / "logs").mkdir(exist_ok=True)
    sys.path.insert(0, str(WT / "scripts"))
    import run_miplib_benchmark as rb
    for p in probs:
        rb.fetch(p, C)
    for rep in range(rep0, rep0 + reps):
        for p in probs:
            for c, envc in (cfgs if rep % 2 == 0 else cfgs[::-1]):
                out = D / "logs" / f"{p}_{c}_{rep}.log"
                if out.exists():
                    continue
                env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **envc)
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{p}.mps"),
                                    "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                out.write_text(r.stderr + "\n" + r.stdout)
                last = r.stdout.strip().splitlines()[-1] if r.stdout.strip() else "{}"
                try:
                    res = json.loads(last)
                except ValueError:
                    res = {"status": "error"}
                rec = {"problem": p, "config": c, "rep": rep, **{k: res.get(k) for k in ("status", "objective", "bound", "time", "nodes", "first_solution_time")}}
                with open(D / "res.jsonl", "a") as f:
                    f.write(json.dumps(rec) + "\n")
                print(json.dumps(rec), flush=True)

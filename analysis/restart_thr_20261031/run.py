"""初回の再スタートの閾値の試作 (2026-10-31、4 スレッド・60 秒)。
土台は hs_pi0_bgw_r (`ENOMOTO_MIP_SUBMIP_HIGHS=-s` + PC_INHERIT + RELIABLE=0 + BG_CUTS_WAIT + `ENOMOTO_MIP_SUBMIP_HIGHS_TRIM=r`)。
使い方: python run.py <設定名,...> <問題,...> <回数> [最初の回]。ログは logs/<問題>_<設定>_<回>.log、結果は run.jsonl に 1 行ずつ足す。
問題ごとに設定の順を回で逆にする。"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
BASE = {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s", "ENOMOTO_MIP_SUBMIP_PC_INHERIT": "1", "ENOMOTO_T_MIP_SUBMIP_RELIABLE": "0",
        "ENOMOTO_MIP_BG_CUTS_WAIT": "1", "ENOMOTO_MIP_SUBMIP_HIGHS_TRIM": "r"}
CFGS = {"def": {}, "r": BASE}
for tag, v in {"f001": "0.001", "f002": "0.002", "f005": "0.005", "f01": "0.01"}.items():
    CFGS[f"r_{tag}"] = dict(BASE, ENOMOTO_T_MIP_RESTART_FIRST_FAC=v)
    CFGS[f"def_{tag}"] = {"ENOMOTO_T_MIP_RESTART_FIRST_FAC": v}
CFGS["r_lpw"] = dict(BASE, ENOMOTO_MIP_RESTART_FIRST_LPW="1")
CFGS["def_lpw"] = {"ENOMOTO_MIP_RESTART_FIRST_LPW": "1"}
CFGS.update(json.loads(os.environ.get("RT_EXTRA_CFGS", "{}")))  # 追加の設定 {"名前": {環境変数}}
if __name__ == "__main__":
    cfgs, probs, reps = sys.argv[1].split(","), sys.argv[2].split(","), int(sys.argv[3])
    rep0 = int(sys.argv[4]) if len(sys.argv) > 4 else 0
    (D / "logs").mkdir(exist_ok=True)
    sys.path.insert(0, str(WT / "scripts"))
    import run_miplib_benchmark as rb
    for p in probs:
        rb.fetch(p, C)
    for rep in range(rep0, rep0 + reps):
        for p in probs:
            for c in (cfgs if rep % 2 == 0 else cfgs[::-1]):
                out = D / "logs" / f"{p}_{c}_{rep}.log"
                if out.exists():
                    continue
                env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **CFGS[c])
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{p}.mps"),
                                    "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                out.write_text(r.stderr + "\n" + r.stdout)
                last = r.stdout.strip().splitlines()[-1] if r.stdout.strip() else "{}"
                with open(D / "run.jsonl", "a") as f:
                    f.write(json.dumps({"prob": p, "cfg": c, "rep": rep, "out": last}, ensure_ascii=False) + "\n")
                print(out.name, last[:160], flush=True)

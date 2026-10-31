"""HiGHS 風のサブ MIP (`ENOMOTO_MIP_SUBMIP_HIGHS=-s`、hs) の文字を 1 つずつ抜いて、どれが遅くするかを切り分ける (2026-10-31、4 スレッド・60 秒)。
使い方: python run.py <設定名,...> <問題,...> <回数> [最初の回]。ログは <問題>_<設定>_<回>.log、結果は run.jsonl に 1 行ずつ足す。"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
CFGS = {"def": {}, "hs": {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s"}}
for ch in "pcgtihf":
    CFGS[f"hs-{ch}"] = {"ENOMOTO_MIP_SUBMIP_HIGHS": f"-s{ch}"}
for k, v in {"hs_trim": "1", "hs_trim_r": "r", "hs_trim_d": "d"}.items():  # 試作 `ENOMOTO_MIP_SUBMIP_HIGHS_TRIM`
    CFGS[k] = {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s", "ENOMOTO_MIP_SUBMIP_HIGHS_TRIM": v}
PI0_BGW = {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s", "ENOMOTO_MIP_SUBMIP_PC_INHERIT": "1", "ENOMOTO_T_MIP_SUBMIP_RELIABLE": "0", "ENOMOTO_MIP_BG_CUTS_WAIT": "1"}
CFGS["hs_pi0_bgw"] = PI0_BGW
CFGS["hs_pi0_bgw_r"] = dict(PI0_BGW, ENOMOTO_MIP_SUBMIP_HIGHS_TRIM="r")
EXTRA = json.loads(os.environ.get("DIAG_EXTRA_CFGS", "{}"))  # 試作の設定 {"名前": {環境変数}}
CFGS.update(EXTRA)
if __name__ == "__main__":
    cfgs, probs, reps = sys.argv[1].split(","), sys.argv[2].split(","), int(sys.argv[3])
    rep0 = int(sys.argv[4]) if len(sys.argv) > 4 else 0
    sys.path.insert(0, str(WT / "scripts"))
    import run_miplib_benchmark as rb
    for p in probs:
        rb.fetch(p, C)
    for rep in range(rep0, rep0 + reps):
        for p in probs:
            for c in (cfgs if rep % 2 == 0 else cfgs[::-1]):
                out = D / f"{p}_{c}_{rep}.log"
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

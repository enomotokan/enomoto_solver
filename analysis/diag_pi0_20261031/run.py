"""サブ MIP の擬似コストの引き継ぎ (pi) と信頼の閾値 0 (r0) の切り分け (2026-10-31、4 スレッド・60 秒)。
使い方: python run.py <設定名,...> <問題,...> <回数> [回の始め]。ログは <問題>_<設定>_<回>.log。
設定は halve_submip_20261030/run.py の CFGS にここの試作を足したもの。"""
import os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
import importlib.util  # noqa: E402
_spec = importlib.util.spec_from_file_location("halve_run", D.parent / "halve_submip_20261030" / "run.py")
_m = importlib.util.module_from_spec(_spec); _spec.loader.exec_module(_m)
BASE = _m.CFGS
HS = {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s"}
CFGS = dict(BASE)
CFGS.update({
    "hs_pi": {**HS, "ENOMOTO_MIP_SUBMIP_PC_INHERIT": "1"},
    "hs_r0": {**HS, "ENOMOTO_T_MIP_SUBMIP_RELIABLE": "0"},
})
EXTRA = {  # 試作 (このコミットのもの)
    "hs_pi0_inh": {**HS, "ENOMOTO_MIP_SUBMIP_PC_INHERIT": "1", "ENOMOTO_T_MIP_SUBMIP_RELIABLE": "0", "ENOMOTO_MIP_SUBMIP_RELIABLE_INHERITED": "1"},
    "hs_sbf25": {**HS, "ENOMOTO_T_MIP_SUBMIP_SB_TIME_FRAC": "0.25"},
    "hs_pi_sbf25": {**HS, "ENOMOTO_MIP_SUBMIP_PC_INHERIT": "1", "ENOMOTO_T_MIP_SUBMIP_SB_TIME_FRAC": "0.25"},
    "hs_pi_sbf25_bgw": {**HS, "ENOMOTO_MIP_SUBMIP_PC_INHERIT": "1", "ENOMOTO_T_MIP_SUBMIP_SB_TIME_FRAC": "0.25", "ENOMOTO_MIP_BG_CUTS_WAIT": "1"},
}
CFGS.update(EXTRA)
if __name__ == "__main__":
    cfgs, probs, reps = sys.argv[1].split(","), sys.argv[2].split(","), int(sys.argv[3])
    r0 = int(sys.argv[4]) if len(sys.argv) > 4 else 0
    for rep in range(r0, r0 + reps):
        for p in probs:
            for c in cfgs:
                out = D / f"{p}_{c}_{rep}.log"
                if out.exists():
                    continue
                env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_DEBUG_SUBMIP_STOP="1", ENOMOTO_MIP_THREADS="4", **CFGS[c])
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{p}.mps"),
                                    "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                out.write_text(r.stderr + "\n" + r.stdout)
                print(out.name, r.stdout.strip().splitlines()[-1][:160] if r.stdout.strip() else "NO OUTPUT", flush=True)

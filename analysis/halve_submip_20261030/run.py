"""カットの LP の本数を半分にする試作 (`ENOMOTO_MIP_CUT_LP_HALVE`) と HiGHS 風のサブ MIP の形を、4 問で試す (2026-10-30、4 スレッド・60 秒)。
使い方: python run.py <設定名,...> <問題,...> <回数>。ログは <問題>_<設定>_<回>.log。"""
import os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
CFGS = {
    "def": {},
    "halve": {"ENOMOTO_MIP_CUT_LP_HALVE": "1"},
    "hnt": {"ENOMOTO_MIP_SUBMIP_HIGHS": "-t"},
    "h1": {"ENOMOTO_MIP_SUBMIP_HIGHS": "1"},
    "rhl": {"ENOMOTO_MIP_RENS_HIGHS": "1", "ENOMOTO_MIP_DIVE_LOCK_ORDER": "1", "ENOMOTO_MIP_SUBMIP_HIGHS": "-t"},
    "halve_hnt": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_MIP_SUBMIP_HIGHS": "-t"},
    "halve_h1": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_MIP_SUBMIP_HIGHS": "1"},
    "frac04": {"ENOMOTO_T_MIP_CUT_TIME_FRAC": "0.4"},
    "halve_frac04": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_TIME_FRAC": "0.4"},
    "hm3": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_LP_CAP_MULT": "3"},
    "hm3_h1": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_LP_CAP_MULT": "3", "ENOMOTO_MIP_SUBMIP_HIGHS": "1"},
    "hm3_frac04": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_LP_CAP_MULT": "3", "ENOMOTO_T_MIP_CUT_TIME_FRAC": "0.4"},
    "hm3_cfl": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_LP_CAP_MULT": "3", "ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "1"},
    "hm3_cfl_h1": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_LP_CAP_MULT": "3", "ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "1", "ENOMOTO_MIP_SUBMIP_HIGHS": "1"},
    "hm3_frac04_h1": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_LP_CAP_MULT": "3", "ENOMOTO_T_MIP_CUT_TIME_FRAC": "0.4", "ENOMOTO_MIP_SUBMIP_HIGHS": "1"},
    "dbl": {"ENOMOTO_MIP_CUT_WIDEN_DOUBLE": "1"},
    "hm3_cfl_dbl": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_LP_CAP_MULT": "3", "ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "1", "ENOMOTO_MIP_CUT_WIDEN_DOUBLE": "1"},
    "hs": {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s"},
    "hm3_cfl_hs": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_LP_CAP_MULT": "3", "ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "1", "ENOMOTO_MIP_SUBMIP_HIGHS": "-s"},
    "bgw": {"ENOMOTO_MIP_BG_CUTS_WAIT": "1"},
    "bgw05": {"ENOMOTO_MIP_BG_CUTS_WAIT": "1", "ENOMOTO_T_MIP_BG_CUTS_WAIT_FRAC": "0.05"},
    "hm3_cfl_hs_bgw": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_T_MIP_CUT_LP_CAP_MULT": "3", "ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "1", "ENOMOTO_MIP_SUBMIP_HIGHS": "-s", "ENOMOTO_MIP_BG_CUTS_WAIT": "1"},
    "hs_r1": {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s", "ENOMOTO_T_MIP_SUBMIP_RELIABLE": "1"},
    "hs_pi0": {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s", "ENOMOTO_MIP_SUBMIP_PC_INHERIT": "1", "ENOMOTO_T_MIP_SUBMIP_RELIABLE": "0"},
    "hs_pi1": {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s", "ENOMOTO_MIP_SUBMIP_PC_INHERIT": "1", "ENOMOTO_T_MIP_SUBMIP_RELIABLE": "1"},
    "hs_pi0_bgw": {"ENOMOTO_MIP_SUBMIP_HIGHS": "-s", "ENOMOTO_MIP_SUBMIP_PC_INHERIT": "1", "ENOMOTO_T_MIP_SUBMIP_RELIABLE": "0", "ENOMOTO_MIP_BG_CUTS_WAIT": "1"},
    "cfl": {"ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "1"},
    "halve_cfl": {"ENOMOTO_MIP_CUT_LP_HALVE": "1", "ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "1"},
}
if __name__ == "__main__":  # bench.py からは設定 (CFGS) だけを読む
    cfgs, probs, reps = sys.argv[1].split(","), sys.argv[2].split(","), int(sys.argv[3])
    for rep in range(reps):
        for p in probs:
            for c in cfgs:
                out = D / f"{p}_{c}_{rep}.log"
                if out.exists():
                    continue
                env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **CFGS[c])
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{p}.mps"),
                                    "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                out.write_text(r.stderr + "\n" + r.stdout)
                print(out.name, r.stdout.strip().splitlines()[-1][:160] if r.stdout.strip() else "NO OUTPUT", flush=True)

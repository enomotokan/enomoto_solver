"""58 問 + 担当 2 問で、既定と対称性の試作 (sym) の対称性の検出だけを比べる (前処理の後で止める)。
symdet/<問題>_<設定>.log に標準エラー、symdet.jsonl に `MIP: symmetry` の行と前処理の時間を残す。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
SYM = {"ENOMOTO_MIP_HO6_SYMWORK": "1", "ENOMOTO_MIP_HO6_SYMREFINE": "1", "ENOMOTO_T_MIP_PRESOLVE_WORK_SYM": "5e8",
       "ENOMOTO_T_MIP_PRESOLVE_WORK_SYM_LOG": "1e9", "ENOMOTO_T_MIP_SYM_TIME": "1"}
CFGS = {"def": {}, "sym": SYM}
dump = "/tmp/claude-0/scratch_symdet.mps"
for p in PROBS + ["uccase12", "unitcal_7"]:
    for c, e in CFGS.items():
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3=dump, ENOMOTO_MIP_DEBUG_STOP_PRESOLVED="1", **e)
        t0 = time.time()
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                            "--time-limit", "120", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=600)
        (D / "symdet" / f"{p}_{c}.log").write_text(r.stderr)
        lines = [l for l in r.stderr.splitlines() if l.startswith("MIP: symmetry")]
        m = re.search(r"presolve finished in ([0-9.]+)s", r.stderr)
        with open(D / "symdet.jsonl", "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "wall": round(time.time() - t0, 1), "presolve": m and float(m.group(1)), "sym": lines}, ensure_ascii=False) + "\n")

"""デバッグ解を作る: 前処理 (対称性の行の後) の問題を書き出し (ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3)、HiGHS で最適に解いて列の順に 1 行 1 値で書く。
python mkdbg.py 出力.sol [種]。書き出しの問題の最適値も表示する (25009.66 より大きければ前処理・対称性の行が最適を落としている)。"""
import os, subprocess, sys, tempfile
from pathlib import Path
import highspy
D = Path(__file__).resolve().parent
WT = D.parents[1]
out = Path(sys.argv[1]); seed = sys.argv[2] if len(sys.argv) > 2 else "1"
mps = WT / ".miplib_cache/neos-4722843-widden.mps"
pre = Path(tempfile.gettempdir()) / f"widden_pre_s{seed}.mps"
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": seed, "ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3": str(pre), "ENOMOTO_MIP_DEBUG_STOP_PRESOLVED": "1", "ENOMOTO_MIP_LOG": "1"})
p = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True)
print("\n".join(l for l in p.stderr.splitlines() if "presolve" in l or "dump" in l.lower())[-2000:])
h = highspy.Highs(); h.setOptionValue("threads", 4); h.setOptionValue("mip_rel_gap", 1e-9); h.setOptionValue("output_flag", False)
h.readModel(str(pre)); h.run()
print("HiGHS status", h.modelStatusToString(h.getModelStatus()), "obj", h.getInfo().objective_function_value)
x = h.getSolution().col_value
out.write_text("\n".join(repr(v) for v in x) + "\n")
print("cols", len(x))

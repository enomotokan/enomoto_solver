"""任意の問題のデバッグ解を作る (mkdbg.py の一般化): python mkdbg2.py 問題 出力.sol [種] [KEY=VAL ...]。
前処理 (対称性の行の後) の問題を書き出し、HiGHS で最適に解いて列の順に 1 行 1 値で書く。この python (sys.executable) のビルドを使う。"""
import os, subprocess, sys, tempfile
from pathlib import Path
import highspy
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
prob, out = sys.argv[1], Path(sys.argv[2]); seed = sys.argv[3] if len(sys.argv) > 3 else "1"
extra = dict(a.split("=", 1) for a in sys.argv[4:])
mps = Path(prob) if prob.endswith(".mps") else rb.fetch(prob, WT / ".miplib_cache")
pre = Path(tempfile.gettempdir()) / f"{prob}_pre_s{seed}.mps"
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(extra)
env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": seed, "ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3": str(pre), "ENOMOTO_MIP_DEBUG_STOP_PRESOLVED": "1", "ENOMOTO_MIP_LOG": "1"})
p = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True)
print("\n".join(l for l in p.stderr.splitlines() if "presolve finished" in l or "written" in l))
h = highspy.Highs(); h.setOptionValue("threads", 4); h.setOptionValue("mip_rel_gap", 1e-9); h.setOptionValue("output_flag", False)
h.readModel(str(pre)); h.run()
print("HiGHS status", h.modelStatusToString(h.getModelStatus()), "obj", h.getInfo().objective_function_value)
out.write_text("\n".join(repr(v) for v in h.getSolution().col_value) + "\n")

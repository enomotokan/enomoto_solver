"""neos-960392 を HiGHS 1.15.1 でログ付きで解く (4 スレッド・60 秒・mip_rel_gap 0)。

使い方: python run_highs.py <ログの出力先> [オプション=値 ...]
例: python run_highs.py out.log mip_heuristic_run_rins=false
"""
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
import highspy  # noqa: E402

log = sys.argv[1]
mps = rb.fetch("neos-960392", Path(".miplib_cache"))
h = highspy.Highs()
h.setOptionValue("output_flag", True)
h.setOptionValue("log_to_console", False)
h.setOptionValue("log_file", log)
h.setOptionValue("threads", 4)
h.setOptionValue("time_limit", 60.0)
h.setOptionValue("mip_rel_gap", 0.0)
for kv in sys.argv[2:]:
    k, v = kv.split("=", 1)
    if v in ("true", "false"):
        v = v == "true"
    else:
        try:
            v = int(v)
        except ValueError:
            try:
                v = float(v)
            except ValueError:
                pass
    h.setOptionValue(k, v)
h.readModel(str(mps))
t = time.perf_counter()
h.run()
w = time.perf_counter() - t
info = h.getInfo()
with open(log, "a") as f:
    f.write(f"WALL {w:.2f} status={h.modelStatusToString(h.getModelStatus())} "
            f"obj={info.objective_function_value} bound={info.mip_dual_bound} nodes={info.mip_node_count}\n")
print(log, f"WALL {w:.2f}", info.objective_function_value, info.mip_dual_bound, info.mip_node_count)

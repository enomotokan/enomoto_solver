"""ENOMOTO が書き出した根のサブ MIP (ENOMOTO_MIP_SUBMIP_DUMP) を HiGHS 1.15.1 で解く。

使い方: python -I solve_dump.py <mps> <ログ> [threads] [time_limit] [オプション=値 ...]
"""
import sys
import time

import highspy

mps, log = sys.argv[1], sys.argv[2]
threads = int(sys.argv[3]) if len(sys.argv) > 3 else 1
tl = float(sys.argv[4]) if len(sys.argv) > 4 else 60.0
h = highspy.Highs()
h.setOptionValue("output_flag", True)
h.setOptionValue("log_to_console", False)
h.setOptionValue("log_file", log)
h.setOptionValue("threads", threads)
h.setOptionValue("time_limit", tl)
h.setOptionValue("mip_rel_gap", 0.0)
for kv in sys.argv[5:]:
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
h.readModel(mps)
t = time.perf_counter()
h.run()
w = time.perf_counter() - t
info = h.getInfo()
line = (f"WALL {w:.2f} status={h.modelStatusToString(h.getModelStatus())} "
        f"obj={info.objective_function_value} bound={info.mip_dual_bound} nodes={info.mip_node_count}")
with open(log, "a") as f:
    f.write(line + "\n")
print(mps, line)

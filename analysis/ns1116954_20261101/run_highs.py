"""HiGHS (highspy) で 1 問をログ付きで解く (2026-11-01)。
使い方: python run_highs.py <mps> <ログ> [オプション=値 ...]  (4 スレッド・60 秒・mip_rel_gap=0 はベンチマークの worker と同じ)"""
import json, sys, time
import highspy
mps, log = sys.argv[1], sys.argv[2]
h = highspy.Highs()
h.setOptionValue("log_file", log)
h.setOptionValue("output_flag", True)
h.setOptionValue("log_to_console", False)
h.setOptionValue("time_limit", 60.0)
h.setOptionValue("threads", 4)
h.setOptionValue("mip_rel_gap", 0.0)
for kv in sys.argv[3:]:
    k, v = kv.split("=", 1)
    for conv in (int, float):
        try:
            v = conv(v); break
        except ValueError:
            pass
    if v in ("true", "false"): v = (v == "true")
    h.setOptionValue(k, v)
h.readModel(mps)
t = time.perf_counter(); h.run(); t = time.perf_counter() - t
i = h.getInfo()
print(json.dumps({"status": h.modelStatusToString(h.getModelStatus()), "objective": i.objective_function_value,
                  "bound": i.mip_dual_bound, "nodes": i.mip_node_count, "time": round(t, 2), "opts": sys.argv[3:]}))

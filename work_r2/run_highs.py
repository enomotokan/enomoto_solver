"""HiGHS を 1 回解く。使い方: run_highs.py MPS 時間 スレッド 種 [option=value ...]"""
import sys, time, highspy
mps, T, th, seed = sys.argv[1], float(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
h = highspy.Highs(); h.setOptionValue("output_flag", False)
h.readModel(mps)
h.setOptionValue("time_limit", T); h.setOptionValue("threads", th); h.setOptionValue("mip_rel_gap", 0.0)
h.setOptionValue("random_seed", seed)
for kv in sys.argv[5:]:
    k, v = kv.split("=")
    try: v = int(v)
    except ValueError:
        try: v = float(v)
        except ValueError: v = {"true": True, "false": False}.get(v, v)
    h.setOptionValue(k, v)
t0 = time.time(); h.run(); dt = time.time() - t0
info = h.getInfo()
print(f"HRESULT t={dt:.1f} status={h.modelStatusToString(h.getModelStatus())} obj={info.objective_function_value} bound={info.mip_dual_bound} nodes={info.mip_node_count}", flush=True)

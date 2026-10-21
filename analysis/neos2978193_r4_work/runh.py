import sys, time, highspy
h = highspy.Highs(); h.setOptionValue("threads", 4); h.setOptionValue("mip_rel_gap", 0.0)
h.setOptionValue("time_limit", float(sys.argv[2]))
for kv in sys.argv[3:]:
    k, v = kv.split("="); 
    try: v = int(v)
    except ValueError:
        try: v = float(v)
        except ValueError: pass
    h.setOptionValue(k, v)
h.readModel(sys.argv[1]); t0 = time.time(); h.run(); t = time.time() - t0
i = h.getInfo()
print(f"RESULT t={t:.2f} status={h.modelStatusToString(h.getModelStatus())} obj={i.objective_function_value} bound={i.mip_dual_bound} nodes={i.mip_node_count}", flush=True)

import sys, time, highspy
h = highspy.Highs()
h.setOptionValue("time_limit", float(sys.argv[2]) if len(sys.argv) > 2 else 60.0)
h.setOptionValue("threads", 4); h.setOptionValue("mip_rel_gap", 0.0)
h.readModel(sys.argv[1]); t = time.perf_counter(); h.run(); print("TIME", time.perf_counter() - t)

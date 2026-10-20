import sys, highspy
T = float(sys.argv[1]) if len(sys.argv) > 1 else 60
h = highspy.Highs(); h.setOptionValue("threads", 4); h.setOptionValue("time_limit", T)
for kv in sys.argv[2:]:
    k, v = kv.split("="); 
    try: v = int(v)
    except ValueError:
        try: v = float(v)
        except ValueError: pass
    h.setOptionValue(k, v)
h.readModel("/home/user/enomoto_solver/.miplib_cache/roll3000.mps"); h.run()

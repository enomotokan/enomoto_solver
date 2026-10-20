import highspy, sys
# usage: highs_run.py <mps> <time_limit> [key=value ...]
h=highspy.Highs(); h.setOptionValue("threads",4); h.setOptionValue("time_limit",float(sys.argv[2])); h.setOptionValue("output_flag",True)
for kv in sys.argv[3:]:
    k,v=kv.split("=",1)
    try: v=int(v)
    except ValueError:
        try: v=float(v)
        except ValueError: pass
    h.setOptionValue(k,v)
h.readModel(sys.argv[1]); h.run()

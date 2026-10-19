import highspy, sys
h=highspy.Highs(); h.setOptionValue("threads",int(__import__("os").environ.get("HT","4"))); h.setOptionValue("time_limit",float(sys.argv[2]))
for kv in sys.argv[3:]:
    k,v=kv.split("=")
    try: v=int(v)
    except ValueError:
        try: v=float(v)
        except ValueError: pass
    h.setOptionValue(k,v)
h.readModel(sys.argv[1]); h.run()

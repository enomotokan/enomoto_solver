import sys, highspy
h=highspy.Highs(); h.setOptionValue('output_flag',True); h.setOptionValue('threads',1)
for kv in sys.argv[3:]:
    k,v=kv.split('='); 
    try: v=int(v)
    except: pass
    h.setOptionValue(k,v)
h.readModel(sys.argv[1]); h.presolve()
h.writePresolvedModel(sys.argv[2])

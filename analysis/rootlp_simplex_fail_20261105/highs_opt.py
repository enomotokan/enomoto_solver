"""HiGHS の双対単体法 (1 スレッド、presolve off) をオプションを変えて解く。使い方: python highs_opt.py 出力.jsonl 時間の上限 名前=値,... MPS..."""
import json, sys, time
from pathlib import Path
import highspy
out, tl, opts = Path(sys.argv[1]), float(sys.argv[2]), sys.argv[3]
for f in sys.argv[4:]:
    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    h.setOptionValue("time_limit", tl)
    h.setOptionValue("threads", 1)
    h.setOptionValue("solver", "simplex")
    h.setOptionValue("simplex_strategy", 1)
    h.setOptionValue("presolve", "off")
    for kv in filter(None, opts.split(",")):
        k, v = kv.split("=")
        try:
            v = int(v)
        except ValueError:
            try:
                v = float(v)
            except ValueError:
                pass
        h.setOptionValue(k, v)
    h.readModel(f)
    t0 = time.time()
    h.run()
    t = time.time() - t0
    info = h.getInfo()
    rec = {"problem": Path(f).stem, "opts": opts, "status": h.modelStatusToString(h.getModelStatus()), "t": round(t, 4),
           "iters": info.simplex_iteration_count, "obj": info.objective_function_value}
    with out.open("a") as fo:
        fo.write(json.dumps(rec) + "\n")
    print(rec, flush=True)

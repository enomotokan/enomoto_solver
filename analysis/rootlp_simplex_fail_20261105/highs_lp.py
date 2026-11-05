"""書き出した根の LP (MPS) を HiGHS の双対単体法で解く (1 スレッド)。
使い方: python highs_lp.py 出力.jsonl 時間の上限 presolve(on/off) MPS...   1 問 1 行 (反復・時間・目的値)。"""
import json, sys, time
from pathlib import Path
import highspy
out, tl, pre = Path(sys.argv[1]), float(sys.argv[2]), sys.argv[3]
for f in sys.argv[4:]:
    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    h.setOptionValue("time_limit", tl)
    h.setOptionValue("threads", 1)
    h.setOptionValue("solver", "simplex")
    h.setOptionValue("simplex_strategy", 1)  # 双対単体法 (逐次)
    h.setOptionValue("presolve", pre)
    h.readModel(f)
    t0 = time.time()
    h.run()
    t = time.time() - t0
    info = h.getInfo()
    rec = {"problem": Path(f).stem, "presolve": pre, "status": h.modelStatusToString(h.getModelStatus()), "t": round(t, 4),
           "iters": info.simplex_iteration_count, "obj": info.objective_function_value,
           "m": h.getNumRow(), "n": h.getNumCol()}
    with out.open("a") as fo:
        fo.write(json.dumps(rec) + "\n")
    print(rec, flush=True)

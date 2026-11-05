"""書き出した根の LP (mps/<問題>.mps) を HiGHS の内点法 + クロスオーバー (solver=ipm, run_crossover=on, presolve=off, 3 スレッド) で解く。
使い方: python -I highs_ipm.py 出力.jsonl 時間の上限 問題...   (presolve を on にするなら環境変数 HIGHS_PRESOLVE=on)"""
import json, os, sys, time
from pathlib import Path
import highspy
HERE = Path(__file__).resolve().parent
out, tl, probs = Path(sys.argv[1]), float(sys.argv[2]), sys.argv[3:]
for p in probs:
    h = highspy.Highs()
    h.setOptionValue("output_flag", True)
    h.setOptionValue("log_to_console", False)
    h.setOptionValue("log_file", str(HERE / "logs" / f"{p}_highs_ipm.log"))
    h.setOptionValue("solver", "ipm")
    h.setOptionValue("run_crossover", "on")
    h.setOptionValue("presolve", os.environ.get("HIGHS_PRESOLVE", "off"))
    h.setOptionValue("threads", 3)
    h.setOptionValue("time_limit", tl)
    h.readModel(str(HERE / "mps" / f"{p}.mps"))
    t0 = time.time()
    h.run()
    t = time.time() - t0
    info = h.getInfo()
    rec = {"problem": p, "status": h.modelStatusToString(h.getModelStatus()), "obj": info.objective_function_value, "t": round(t, 3),
           "ipm_iters": info.ipm_iteration_count, "xo_iters": info.crossover_iteration_count, "simplex_iters": info.simplex_iteration_count,
           "basis_valid": info.basis_validity, "presolve": os.environ.get("HIGHS_PRESOLVE", "off")}
    log = (HERE / "logs" / f"{p}_highs_ipm.log").read_text(errors="replace")
    for l in log.splitlines():
        if "IPX" in l and ("time" in l.lower()) or "Crossover" in l or "crossover" in l:
            pass
    with out.open("a") as f:
        f.write(json.dumps(rec) + "\n")
    print(rec, flush=True)

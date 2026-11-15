"""PRESOLVE2: HiGHS (highspy 1.15.1) の前処理だけを回し、規則ごとの減り (presolve_rule_logging) を記録する。
使い方: python highs_rules.py 問題 [presolve_rule_off のビット値]
出力: hlog/<問題>[.off<ビット>].log (HiGHS のログそのまま)、最後に 1 行の JSON (前処理後の行・列・非零、秒)。"""
import json, sys, time
from pathlib import Path
import highspy
D = Path(__file__).resolve().parent
WT = D.parents[1]
p = sys.argv[1]
off = int(sys.argv[2]) if len(sys.argv) > 2 else 0
log = D / "hlog" / (f"{p}.off{off}.log" if off else f"{p}.log")
log.unlink(missing_ok=True)
h = highspy.Highs()
h.setOptionValue("output_flag", True)
h.setOptionValue("log_to_console", False)
h.setOptionValue("log_file", str(log))
h.setOptionValue("presolve_rule_logging", True)
h.setOptionValue("log_dev_level", 1)
h.setOptionValue("presolve_rule_off", off)
h.setOptionValue("threads", 1)
h.readModel(str(WT / ".miplib_cache" / f"{p}.mps"))
t0 = time.time()
h.presolve()
t = time.time() - t0
lp = h.getPresolvedLp()
print(json.dumps({"problem": p, "off": off, "rows": lp.num_row_, "cols": lp.num_col_, "nnz": len(lp.a_matrix_.index_), "secs": round(t, 2)}), flush=True)

"""HiGHS 1.15.1 の前処理の規則を 1 つずつ切って、前処理後の行・列・2 値・一般整数・連続の数と LP の値を出す (analysis/presolvegap_20261113.md)。
python hrules.py <問題 ...>  (highspy 1.15.1 の venv で)"""
import sys, highspy, numpy as np
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
NAMES = {0: "EmptyRow", 1: "SingletonRow", 2: "RedundantRow", 3: "EmptyCol", 4: "FixedCol", 5: "DominatedCol", 6: "ForcingRow", 7: "ForcingCol",
         8: "FreeColSubstitution", 9: "DoubletonEquation", 10: "DependentEquations", 11: "DependentFreeCols", 12: "Aggregator",
         13: "ParallelRowsAndCols", 14: "?14", 15: "Probing", 16: "Enumeration"}
def run(p, off):
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("threads", 1)
    h.readModel(str(WT / ".miplib_cache" / f"{p}.mps")); h.setOptionValue("presolve_rule_off", off)
    h.presolve(); lp = h.getPresolvedLp()
    integ = list(lp.integrality_) if len(lp.integrality_) else [highspy.HighsVarType.kContinuous] * lp.num_col_
    lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
    isint = np.array([v == highspy.HighsVarType.kInteger for v in integ])
    nbin = int(np.sum(isint & (lo == 0) & (up == 1))); nint = int(np.sum(isint)) - nbin; ncont = lp.num_col_ - int(np.sum(isint))
    g = highspy.Highs(); g.setOptionValue("output_flag", False); g.setOptionValue("threads", 1); g.passModel(lp)
    g.setOptionValue("solve_relaxation", True); g.run()
    return f"rows {lp.num_row_} cols {lp.num_col_} (bin {nbin} gen-int {nint} cont {ncont}) nnz {lp.a_matrix_.index_.__len__()} LP {g.getInfo().objective_function_value:.4f}"
for p in sys.argv[1:]:
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(str(WT / ".miplib_cache" / f"{p}.mps")); lp = h.getLp()
    print(f"== {p}: original rows {lp.num_row_} cols {lp.num_col_}")
    print("  all on       ", run(p, 0))
    for b in range(0, 17):
        print(f"  off {b:2d} {NAMES[b]:20s}", run(p, 1 << b))
    print("  off 12+13    ", run(p, (1 << 12) | (1 << 13)))

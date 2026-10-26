# HiGHS の前処理の規則を 1 つずつ止めて前処理後の大きさと LP 緩和の値を比べる
# 番号 -1 は止められる規則 (6〜19) を全部止める
# 使い方: python3 hoff.py <mps> [規則の番号...]  (番号は presolve_rule_off のビット、all は何も止めない。なしは規則ごと)
import sys, time, highspy
mps = sys.argv[1]
rules = [None if x == "all" else int(x) for x in sys.argv[2:]] or [None, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18]
for r in rules:
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("threads", 1)
    if r is not None:
        h.setOptionValue("presolve_rule_off", 1 << r if r >= 0 else (1 << 20) - 64)
    h.readModel(mps); t = time.time(); h.presolve(); lp = h.getPresolvedLp(); t = time.time() - t
    g = highspy.Highs(); g.setOptionValue("output_flag", False); g.setOptionValue("presolve", "off"); g.setOptionValue("solve_relaxation", True)
    g.passModel(lp); g.run()
    z = g.getInfo().objective_function_value if g.getModelStatus() == highspy.HighsModelStatus.kOptimal else None
    print(f"off={r} rows={lp.num_row_} cols={lp.num_col_} nnz={len(lp.a_matrix_.index_)} lp={z} t={t:.1f}", flush=True)

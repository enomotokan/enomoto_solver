# HiGHS の前処理の規則を 1 つずつ止めて前処理後の大きさを比べる
# 使い方: python3 hoff.py <mps> [規則の番号...]  (番号は presolve_rule_off のビット。なしは全部の規則)
import sys, time, highspy
mps = sys.argv[1]
rules = [int(x) for x in sys.argv[2:]] or [None, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18]
for r in rules:
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.setOptionValue("threads", 4)
    if r is not None:
        h.setOptionValue("presolve_rule_off", 1 << r)
    h.readModel(mps); t = time.time(); h.presolve(); lp = h.getPresolvedLp()
    print(f"off={r} rows={lp.num_row_} cols={lp.num_col_} nnz={len(lp.a_matrix_.index_)} t={time.time()-t:.1f}", flush=True)

"""ruleoff.py <mps>: HiGHS の前処理を規則を 1 つずつ止めてかけ、前処理の後の行・列・非零を出す (列を消している規則を探す)"""
import sys, highspy
names = {6:'forcing row',7:'forcing col',8:'free col subst',9:'doubleton eq',10:'dep eq',11:'dep free col',12:'aggregator',13:'parallel rows/cols',14:'sparsify',15:'probing',16:'enumeration',17:'dual fixing',18:'col stuffing',19:'initial sweep'}
for r in [None] + list(names):
    h = highspy.Highs(); h.setOptionValue('output_flag', False); h.readModel(sys.argv[1])
    if r is not None: h.setOptionValue('presolve_rule_off', 1 << r)
    h.presolve(); lp = h.getPresolvedLp()
    print(f"{'(none)' if r is None else names[r]:20s} rows {lp.num_row_} cols {lp.num_col_} nnz {len(lp.a_matrix_.value_)}", flush=True)

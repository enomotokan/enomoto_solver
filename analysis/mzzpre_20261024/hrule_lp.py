"""HiGHS の前処理をオプションつきでかけ、前処理後の大きさと LP 緩和の値を出す:
python hrule_lp.py <mps> [キー=値 ...] [--write 出力.mps]。最後の行に JSON。"""
import json, sys, time
import highspy
args = sys.argv[2:]
out = None
if "--write" in args:
    i = args.index("--write"); out = args[i + 1]; args = args[:i] + args[i + 2:]
h = highspy.Highs()
h.setOptionValue("threads", 4)
for kv in args:
    k, v = kv.split("=")
    for cast in (int, float, str):
        try:
            if h.setOptionValue(k, cast(v)) == highspy.HighsStatus.kOk: break
        except Exception:
            pass
h.readModel(sys.argv[1])
t = time.perf_counter(); h.presolve(); tp = time.perf_counter() - t
lp = h.getPresolvedLp()
h2 = highspy.Highs(); h2.setOptionValue("output_flag", False)
h2.passModel(lp)
if out:
    h2.writeModel(out)
nint = sum(1 for v in lp.integrality_ if v != highspy.HighsVarType.kContinuous) if len(lp.integrality_) else 0
h2.setOptionValue("solver", "simplex")
# 整数性を外して LP 緩和を解く
lp2 = h2.getLp(); lp2.integrality_ = []
h3 = highspy.Highs(); h3.setOptionValue("output_flag", False); h3.setOptionValue("threads", 4); h3.passModel(lp2)
t = time.perf_counter(); h3.run(); tl = time.perf_counter() - t
obj = h3.getInfo().objective_function_value
print(json.dumps({"opts": args, "presolve_s": round(tp, 1), "rows": lp.num_row_, "cols": lp.num_col_, "nnz": lp.a_matrix_.num_nz_ if hasattr(lp.a_matrix_, "num_nz_") else len(lp.a_matrix_.index_),
                  "int": nint, "offset": lp.offset_, "lp": obj, "lp_s": round(tl, 1)}), flush=True)

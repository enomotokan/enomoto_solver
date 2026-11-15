"""classify.py の全列挙の点を独立に確かめる: 2 値の点に連続列 0 を足した点の元の問題での違反と目的値 (連続列の寄与 ~1e-6 は除く)。
また HiGHS (厳しい許容誤差) で 2 値を固定して解いた値。"""
import sys, json
sys.path.insert(0, "analysis/fuzz_tiny_classify_20261103")
sys.argv = ["classify.py", "0"]
exec(open("analysis/fuzz_tiny_classify_20261103/classify.py").read().split("N = int(sys.argv[2])")[0])
TIGHT = {"primal_feasibility_tolerance": 1e-10, "mip_feasibility_tolerance": 1e-10, "presolve": "off"}
for l in open("analysis/fuzz_tiny_classify_20261103/classify_400.jsonl"):
    d = json.loads(l); lp = gen(d["seed"]); lo, up, c, it, rows, rlo, rup = lp; nb = sum(it)
    out = [d["seed"]]
    for key in ("exact_int_point", "tol6_int_point", "ours_int_point", "highs_int_point"):
        p = d[key]; x = list(map(float, p)) + [0.0] * (len(lo) - nb)
        lo2 = [x[j] if it[j] else lo[j] for j in range(len(lo))]; up2 = [x[j] if it[j] else up[j] for j in range(len(lo))]
        st, z, _ = build(lp, lo=lo2, up=up2, opts=TIGHT)
        out.append("%s: viol(y=0) %.1e obj(y=0) %.2f | HiGHS厳しく固定 %s %s" % (key.split("_")[0], max(viol(lp, x)), sum(ci*xi for ci, xi in zip(c, x)), st, z))
    print(" ; ".join(map(str, out)))

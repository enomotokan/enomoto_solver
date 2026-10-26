"""書き出したカット LP (`ENOMOTO_MIP_DEBUG_DUMP_CUT_LP`) を HiGHS で解く: python solve_dump.py <prefix> [秒] [cold|warm] [dual|primal]
出発の基底は <prefix>.bas (基底列の番号と直近の解 x) から作る (ENOMOTO の nb_hint と同じ規則で非基底の置き場所を決める)。"""
import sys, time, math
import highspy

prefix = sys.argv[1]
T = float(sys.argv[2]) if len(sys.argv) > 2 else 60.0
mode = sys.argv[3] if len(sys.argv) > 3 else "warm"
alg = sys.argv[4] if len(sys.argv) > 4 else "dual"

h = highspy.Highs()
h.setOptionValue("output_flag", True)
h.setOptionValue("log_to_console", True)
h.setOptionValue("presolve", "off")
h.setOptionValue("solver", "simplex")
h.setOptionValue("simplex_strategy", 1 if alg == "dual" else 4)
h.setOptionValue("threads", 1)
h.setOptionValue("time_limit", T)
st = h.readModel(prefix + ".mps")
lp = h.getLp()
n, m = lp.num_col_, lp.num_row_
print(f"read {prefix}.mps: {st} n={n} m={m}", flush=True)

if mode == "warm":
    lines = open(prefix + ".bas").read().splitlines()
    b = [int(t) for t in lines[1].split()]
    x = [float(t) for t in lines[2].split()]
    assert len(b) == m and len(x) == n, (len(b), len(x))
    basic = set(b)
    cl = list(lp.col_lower_); cu = list(lp.col_upper_)
    rl = list(lp.row_lower_); ru = list(lp.row_upper_)
    am = lp.a_matrix_
    start, index, value = list(am.start_), list(am.index_), list(am.value_)
    act = [0.0] * m
    assert "kColwise" in str(am.format_)
    for j in range(n):
        for k in range(start[j], start[j + 1]):
            act[index[k]] += value[k] * x[j]
    at = lambda v, bnd: math.isfinite(bnd) and abs(v - bnd) <= 1e-9 * (1.0 + abs(bnd))
    B = highspy.HighsBasis()
    B.valid = True
    cs, rs = [], []
    nstat = {"L": 0, "U": 0, "Z": 0, "B": 0}
    for j in range(n):
        if j in basic:
            cs.append(highspy.HighsBasisStatus.kBasic); nstat["B"] += 1
        elif at(x[j], cl[j]):
            cs.append(highspy.HighsBasisStatus.kLower); nstat["L"] += 1
        elif at(x[j], cu[j]):
            cs.append(highspy.HighsBasisStatus.kUpper); nstat["U"] += 1
        elif not math.isfinite(cl[j]) and not math.isfinite(cu[j]):
            cs.append(highspy.HighsBasisStatus.kZero); nstat["Z"] += 1
        elif math.isfinite(cl[j]):
            cs.append(highspy.HighsBasisStatus.kLower); nstat["L"] += 1
        else:
            cs.append(highspy.HighsBasisStatus.kUpper); nstat["U"] += 1
    rstat = {"L": 0, "U": 0, "B": 0}
    for i in range(m):
        if n + i in basic:
            rs.append(highspy.HighsBasisStatus.kBasic); rstat["B"] += 1
        elif at(act[i], ru[i]):
            rs.append(highspy.HighsBasisStatus.kUpper); rstat["U"] += 1
        elif at(act[i], rl[i]):
            rs.append(highspy.HighsBasisStatus.kLower); rstat["L"] += 1
        elif math.isfinite(ru[i]):
            rs.append(highspy.HighsBasisStatus.kUpper); rstat["U"] += 1
        else:
            rs.append(highspy.HighsBasisStatus.kLower); rstat["L"] += 1
    B.col_status = cs
    B.row_status = rs
    print(f"basis: cols {nstat} rows {rstat}", flush=True)
    print("setBasis:", h.setBasis(B), flush=True)

t0 = time.perf_counter()
h.run()
t = time.perf_counter() - t0
info = h.getInfo()
print(f"RESULT {prefix} mode={mode} alg={alg}: status={h.modelStatusToString(h.getModelStatus())} iters={info.simplex_iteration_count} time={t:.3f}s obj={info.objective_function_value:.10e}", flush=True)

"""書き出したノードの LP (<prefix>.mps / .bas) を HiGHS の双対単体法で同じ出発の基底から解く (1 スレッド、前処理なし)。
基底の作り方は analysis/lp_hang_20261021/solve_dump.py と同じ (非基底の置き場所は直近の解 x から)。
使い方: python -I highs_warm.py <prefix> [opt=val ...]   出力: 1 行の JSON (iters, secs, status, obj)"""
import sys, time, math, json
import highspy

prefix = sys.argv[1]
h = highspy.Highs()
h.setOptionValue("output_flag", False)
h.setOptionValue("presolve", "off")
h.setOptionValue("solver", "simplex")
h.setOptionValue("simplex_strategy", 1)
h.setOptionValue("threads", 1)
h.setOptionValue("time_limit", 120.0)
for kv in sys.argv[2:]:
    k, v = kv.split("=", 1)
    try:
        v = int(v)
    except ValueError:
        try:
            v = float(v)
        except ValueError:
            pass
    h.setOptionValue(k, v)
h.readModel(prefix + ".mps")
lp = h.getLp()
n, m = lp.num_col_, lp.num_row_
lines = open(prefix + ".bas").read().splitlines()
b = [int(t) for t in lines[1].split()]
x = [float(t) for t in lines[2].split()]
basic = set(b)
cl = list(lp.col_lower_); cu = list(lp.col_upper_)
rl = list(lp.row_lower_); ru = list(lp.row_upper_)
am = lp.a_matrix_
start, index, value = list(am.start_), list(am.index_), list(am.value_)
act = [0.0] * m
for j in range(n):
    for k in range(start[j], start[j + 1]):
        act[index[k]] += value[k] * x[j]
at = lambda v, bnd: math.isfinite(bnd) and abs(v - bnd) <= 1e-9 * (1.0 + abs(bnd))
S = highspy.HighsBasisStatus
cs, rs = [], []
for j in range(n):
    if j in basic:
        cs.append(S.kBasic)
    elif at(x[j], cl[j]):
        cs.append(S.kLower)
    elif at(x[j], cu[j]):
        cs.append(S.kUpper)
    elif not math.isfinite(cl[j]) and not math.isfinite(cu[j]):
        cs.append(S.kZero)
    elif math.isfinite(cl[j]) and (not math.isfinite(cu[j]) or abs(x[j] - cl[j]) <= abs(x[j] - cu[j])):
        cs.append(S.kLower)
    else:
        cs.append(S.kUpper)
for i in range(m):
    if n + i in basic:
        rs.append(S.kBasic)
    elif at(act[i], ru[i]):
        rs.append(S.kUpper)
    elif at(act[i], rl[i]):
        rs.append(S.kLower)
    elif math.isfinite(ru[i]):
        rs.append(S.kUpper)
    else:
        rs.append(S.kLower)
B = highspy.HighsBasis()
B.valid = True
B.col_status = cs
B.row_status = rs
sb = h.setBasis(B)
t0 = time.perf_counter()
h.run()
t = time.perf_counter() - t0
info = h.getInfo()
print(json.dumps({"iters": info.simplex_iteration_count, "secs": round(t, 4), "status": h.modelStatusToString(h.getModelStatus()),
                  "obj": info.objective_function_value, "setbasis": str(sb), "m": m, "n": n}))

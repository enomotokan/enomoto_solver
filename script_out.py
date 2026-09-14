
import json, math, os, sys, time
sys.path.insert(0, r"c:\\Users\\enomo\\Desktop\\2026\\python\\ENOMOTO-Solver\\python")
import highspy
from enomoto_solver import _core

mps_path = sys.argv[1]
h = highspy.Highs()
h.setOptionValue("output_flag", False)
status = h.readModel(mps_path)
if "kOk" not in str(status):
    print(json.dumps({"error": f"readModel failed: {status}"}))
    sys.exit(0)
lp = h.getLp()

m = _core.PyModel()
n = lp.num_col_
for j in range(n):
    lb = float(lp.col_lower_[j]); ub = float(lp.col_upper_[j])
    if lb > ub:
        lb, ub = ub, lb
    is_int = len(lp.integrality_) > j and int(lp.integrality_[j]) != 0
    m.add_variable("integer" if is_int else "continuous", lb, ub)

obj_coeffs = [(j, float(c)) for j, c in enumerate(lp.col_cost_) if c != 0.0]
sense = "maximize" if "kMaximize" in str(lp.sense_) else "minimize"
m.set_objective(obj_coeffs, float(lp.offset_), sense)

n_rows = lp.num_row_
rows = [[] for _ in range(n_rows)]
am = lp.a_matrix_
start, index, value = list(am.start_), list(am.index_), list(am.value_)
if "kColwise" in str(am.format_):
    for j in range(n):
        for k in range(start[j], start[j + 1]):
            if value[k] != 0.0:
                rows[index[k]].append((j, value[k]))
else:
    for i in range(n_rows):
        for k in range(start[i], start[i + 1]):
            if value[k] != 0.0:
                rows[i].append((index[k], value[k]))

for i in range(n_rows):
    lo, hi = lp.row_lower_[i], lp.row_upper_[i]
    terms = rows[i]
    if math.isinf(lo) and math.isinf(hi):
        continue
    if not math.isinf(lo) and abs(hi - lo) < 1e-12:
        m.add_constraint(terms, "==", float(lo))
        continue
    if not math.isinf(hi):
        m.add_constraint(terms, "<=", float(hi))
    if not math.isinf(lo):
        m.add_constraint(terms, ">=", float(lo))

t0 = time.perf_counter()
out = m.solve(root_solver=None)
elapsed = time.perf_counter() - t0
print(json.dumps({"status": out["status"], "time_s": elapsed}))

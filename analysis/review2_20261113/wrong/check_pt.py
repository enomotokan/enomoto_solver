"""DUALEQ の前後の問題 (ENOMOTO_MIP_DEBUG_DUMP_DUALEQ の書き出し) を HiGHS で解き、等式にした後の問題 (eq) の実行可能な点が
TREE2_FORCEQ の 2 項等式 (src/mip/mod.rs の tree2_forceq_rows と同じ規則) を満たすかを確かめる。python check_pt.py <接頭辞> [秒]"""
import sys
import highspy
import numpy as np
pre = sys.argv[1]; tl = float(sys.argv[2]) if len(sys.argv) > 2 else 120
def load(f):
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(f); return h
def solve(h, mip=True):
    h.setOptionValue("time_limit", tl)
    if mip: h.setOptionValue("mip_max_improving_sols", 1)
    h.run(); return h.getModelStatus(), h.getInfo().objective_function_value, np.array(h.getSolution().col_value)
def data(h):
    lp = h.getLp(); m, n = lp.num_row_, lp.num_col_
    a = lp.a_matrix_; st, ix, v = list(a.start_), list(a.index_), list(a.value_)
    rows = [[] for _ in range(m)]
    for j in range(n):
        for k in range(st[j], st[j + 1]): rows[ix[k]].append((j, v[k]))
    integ = [int(x) == 1 for x in lp.integrality_] if len(lp.integrality_) else [False] * n
    return rows, list(lp.row_lower_), list(lp.row_upper_), list(lp.col_lower_), list(lp.col_upper_), integ
def forceq(rows, rlo, rup, clo, cup, isint):
    out = []; seen = set()
    for i, r in enumerate(rows):
        if rlo[i] != rup[i] or not np.isfinite(rlo[i]) or len(r) < 2 or len(r) > 1000: continue
        b = rlo[i]; y = None; ok = True
        for j, a in r:
            if a == 0: continue
            if isint[j] and clo[j] == 0 and cup[j] == 1:
                if y is not None: ok = False; break
                y = (j, a)
            elif isint[j] or not np.isfinite(clo[j]) or not np.isfinite(cup[j]) or clo[j] == cup[j]: ok = False; break
        if y is None or not ok: continue
        yj, ay = y; lo = hi = 0.0; mag = abs(b) + abs(ay)
        for j, a in r:
            if j == yj or a == 0: continue
            l, u = (a * clo[j], a * cup[j]) if a > 0 else (a * cup[j], a * clo[j]); lo += l; hi += u; mag += abs(l) + abs(u)
        tol = 1e-9 * max(mag, 1); r0, r1 = b, b - ay
        if abs(r0 - lo) <= tol and abs(r1 - hi) <= tol: low0 = True
        elif abs(r0 - hi) <= tol and abs(r1 - lo) <= tol: low0 = False
        else: continue
        for j, a in r:
            if j == yj or a == 0 or (j, yj) in seen: continue
            seen.add((j, yj))
            atl, ath = (clo[j], cup[j]) if a > 0 else (cup[j], clo[j])
            v0, v1 = (atl, ath) if low0 else (ath, atl)
            out.append((i, j, yj, v0, v1))
    return out
for tag in ("eq", "pre"):
    h = load(f"{pre}_{tag}.mps"); d = data(h)
    print(tag, "行", len(d[0]), "列", len(d[3]))
    s, obj, x = solve(h)
    print(tag, "HiGHS:", h.modelStatusToString(s), obj)
    if len(x) == 0: continue
    rows, rlo, rup, clo, cup, isint = d
    fq = forceq(*d)
    bad = [(i, j, yj, v0, v1, x[j], x[yj]) for i, j, yj, v0, v1 in fq if abs(x[j] - (v0 + (v1 - v0) * x[yj])) > 1e-6 * max(1, abs(v0), abs(v1))]
    print(tag, "FORCEQ の等式", len(fq), "破る", len(bad))
    for t in bad[:10]:
        i = t[0]; print("  行", i, "x", t[1], "y", t[2], "v0 v1", t[3], t[4], "x*", t[5], "y*", t[6], "行の中身", rows[i][:8], rlo[i], rup[i])

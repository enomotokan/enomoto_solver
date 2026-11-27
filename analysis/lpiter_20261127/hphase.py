"""HiGHS の双対単体法の 1 反復の内訳 (highs_analysis_level=8 の SimplexInner の計時) を取る。1 スレッド・前処理なし。
使い方: python -I hphase.py rel <LP の MPS> [秒]          冷えた始動 (LP 緩和)
        python -I hphase.py warm <prefix>                  書き出したノードの LP (<prefix>.mps / .bas) を同じ出発の基底から
                                                            (基底の作り方は analysis/node_lp_20261106/highs_warm.py と同じ)
        timer=0 を足すと計時なし (計時の重さを見る)。HPHASE_LEVEL で解析の段階 (既定 8、12 で FTRAN などの密度の集計も)、
        HPHASE_RAW=<ファイル> で HiGHS の出力をそのまま書く。
出力: 最後の行に 1 行の JSON (iters, secs, status, phases = {段階: 秒})。"""
import io, json, math, os, re, sys, tempfile, time
import highspy

mode, arg = sys.argv[1], sys.argv[2]
rest = sys.argv[3:]
timer = "timer=0" not in rest
rest = [r for r in rest if r != "timer=0"]
tl = float(rest[0]) if (mode == "rel" and rest) else 600.0
h = highspy.Highs()
h.setOptionValue("output_flag", timer)
h.setOptionValue("log_to_console", timer)
h.setOptionValue("presolve", "off")
h.setOptionValue("solver", "simplex")
h.setOptionValue("simplex_strategy", 1)
h.setOptionValue("threads", 1)
h.setOptionValue("time_limit", tl)
if timer:
    h.setOptionValue("highs_analysis_level", int(os.environ.get("HPHASE_LEVEL", "8")))
h.readModel(arg + ".mps" if mode == "warm" else arg)
if mode == "rel":
    lp = h.getLp()
    if len(lp.integrality_):
        lp.integrality_ = []
        h.passModel(lp)
else:
    lp = h.getLp()
    n, m = lp.num_col_, lp.num_row_
    lines = open(arg + ".bas").read().splitlines()
    b = [int(t) for t in lines[1].split()]
    x = [float(t) for t in lines[2].split()]
    basic = set(b)
    cl = list(lp.col_lower_); cu = list(lp.col_upper_); rl = list(lp.row_lower_); ru = list(lp.row_upper_)
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
        if j in basic: cs.append(S.kBasic)
        elif at(x[j], cl[j]): cs.append(S.kLower)
        elif at(x[j], cu[j]): cs.append(S.kUpper)
        elif not math.isfinite(cl[j]) and not math.isfinite(cu[j]): cs.append(S.kZero)
        elif math.isfinite(cl[j]) and (not math.isfinite(cu[j]) or abs(x[j] - cl[j]) <= abs(x[j] - cu[j])): cs.append(S.kLower)
        else: cs.append(S.kUpper)
    for i in range(m):
        if n + i in basic: rs.append(S.kBasic)
        elif at(act[i], ru[i]): rs.append(S.kUpper)
        elif at(act[i], rl[i]): rs.append(S.kLower)
        elif math.isfinite(ru[i]): rs.append(S.kUpper)
        else: rs.append(S.kLower)
    B = highspy.HighsBasis(); B.valid = True; B.col_status = cs; B.row_status = rs
    h.setBasis(B)
# C の printf の出力を一時ファイルに受ける
sys.stdout.flush()
fd = os.dup(1)
tmp = tempfile.TemporaryFile(mode="w+b")
os.dup2(tmp.fileno(), 1)
t0 = time.perf_counter(); h.run(); t = time.perf_counter() - t0
sys.stdout.flush()
os.dup2(fd, 1)
tmp.seek(0); txt = tmp.read().decode(errors="replace")
ph = {}; calls = {}
for l in txt.splitlines():
    mm = re.match(r"SimplexInner-time\s+(\w+)\s+:\s+([\d.e+-]+) \(.*\):\s+(\d+)", l)
    if mm:
        ph[mm.group(1)] = ph.get(mm.group(1), 0.0) + float(mm.group(2)); calls[mm.group(1)] = calls.get(mm.group(1), 0) + int(mm.group(3))
if os.environ.get("HPHASE_RAW"):
    open(os.environ["HPHASE_RAW"], "w").write(txt)
info = h.getInfo()
print(json.dumps({"iters": info.simplex_iteration_count, "secs": round(t, 4), "status": h.modelStatusToString(h.getModelStatus()),
                  "obj": info.objective_function_value, "timer": timer, "phases": ph, "calls": calls}))

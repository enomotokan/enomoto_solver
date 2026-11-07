"""MIPLIB の問題 (整数列と連続列が混ざってよい) の整数列を、格子の基底で置き換えた形 x = U^T ν (U は単模、ν は上下限なしの整数) に
書き直して、元の形と ENOMOTO (既定) で比べる (2026-11-07、AHL 2000 の書き直しを最適化問題・スラックつきに広げる見積もり)。
U は埋め込み [I | N A_eq] (A_eq は等式の行の整数列の係数) の行を LLL で簡約したもの。最初の n - rank 本が A_eq の核 (A_eq u = 0)。
連続列はそのまま残す。整数列の上下限は行 lo_j ≤ Σ_k U_kj ν_k ≤ up_j になる。
使い方: python ahl2.py out.jsonl T 問題... (問題は .miplib_cache の名前)"""
import json, os, subprocess, sys, time
from pathlib import Path
import numpy as np, highspy
from fpylll import IntegerMatrix, LLL
D = Path(__file__).resolve().parent
WT = D.parents[1]
TMP = Path(os.environ.get("AHL_TMP", "/tmp/ahl")); TMP.mkdir(exist_ok=True)
INF = float("inf")

def read(name):
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(str(WT / ".miplib_cache" / f"{name}.mps"))
    lp = h.getLp(); n, m = lp.num_col_, lp.num_row_
    it = [int(v) for v in lp.integrality_] if len(lp.integrality_) else [0] * n
    st = np.array(lp.a_matrix_.start_); idx = np.array(lp.a_matrix_.index_); val = np.array(lp.a_matrix_.value_)
    rows = [[] for _ in range(m)]
    for j in range(n):
        for p in range(st[j], st[j + 1]): rows[idx[p]].append((j, float(val[p])))
    sense = 1 if lp.sense_ == highspy.ObjSense.kMinimize else -1
    cl = [float(v) if v > -1e30 else -INF for v in lp.col_lower_]; cu = [float(v) if v < 1e30 else INF for v in lp.col_upper_]
    rl = [float(v) if v > -1e30 else -INF for v in lp.row_lower_]; ru = [float(v) if v < 1e30 else INF for v in lp.row_upper_]
    return dict(n=n, m=m, it=it, cost=[float(sense * c) for c in lp.col_cost_], cl=cl, cu=cu, rows=rows, rl=rl, ru=ru, off=float(sense * lp.offset_))

def write_mps(path, cols, rows, rl, ru, off=0.0):
    """cols: [(名前, 整数か, 下限, 上限, 費用)]、rows: [[(列, 係数)]]"""
    L = ["NAME x", "ROWS", " N obj"]
    for i in range(len(rows)):
        k = "E" if rl[i] == ru[i] else ("G" if ru[i] == INF else "L")
        L.append(f" {k} r{i}")
    L.append("COLUMNS")
    cc = [[] for _ in cols]
    for i, r in enumerate(rows):
        for j, a in r: cc[j].append((i, a))
    inint = False
    for j, (nm, isint, lo, up, c) in enumerate(cols):
        if isint != inint:
            L.append(f" M{j} 'MARKER' '{'INTORG' if isint else 'INTEND'}'"); inint = isint
        L.append(f" {nm} obj {c!r}")
        for i, a in cc[j]: L.append(f" {nm} r{i} {a!r}")
    if inint: L.append(" Mend 'MARKER' 'INTEND'")
    L.append("RHS")
    if off: L.append(f" rhs obj {-off!r}")
    for i in range(len(rows)):
        k = "E" if rl[i] == ru[i] else ("G" if ru[i] == INF else "L")
        v = rl[i] if k in "EG" else ru[i]
        if v != 0: L.append(f" rhs r{i} {v!r}")
    rng = [(i, ru[i] - rl[i]) for i in range(len(rows)) if rl[i] != ru[i] and rl[i] > -INF and ru[i] < INF]
    if rng: L.append("RANGES"); L += [f" rng r{i} {v!r}" for i, v in rng]
    L.append("BOUNDS")
    for nm, isint, lo, up, c in cols:
        if lo == -INF and up == INF: L.append(f" FR bnd {nm}")
        else:
            if lo == -INF: L.append(f" MI bnd {nm}")
            elif lo != 0: L.append(f" LO bnd {nm} {lo!r}")
            if up < INF: L.append(f" UP bnd {nm} {up!r}")
    L.append("ENDATA"); Path(path).write_text("\n".join(L) + "\n")

def reform(P, N=1000):
    n = P["n"]; I = [j for j in range(n) if P["it"][j] == 1]; C = [j for j in range(n) if P["it"][j] != 1]
    pos = {j: k for k, j in enumerate(I)}
    eq = [i for i in range(P["m"]) if P["rl"][i] == P["ru"][i]]
    # 等式の整数列の係数 (整数に丸められるものだけ使う)
    Aeq = []
    for i in eq:
        r = [0] * len(I); ok = True
        for j, a in P["rows"][i]:
            if j in pos:
                if abs(a - round(a)) > 1e-9: ok = False; break
                r[pos[j]] = int(round(a))
        if ok and any(r): Aeq.append(r)
    ni = len(I)
    B = [[1 if k == j else 0 for k in range(ni)] + [N * Aeq[i][j] for i in range(len(Aeq))] for j in range(ni)]
    M = IntegerMatrix.from_matrix(B); LLL.reduction(M)
    U = [[M[r, k] for k in range(ni)] for r in range(ni)]
    ker = sum(1 for r in range(ni) if all(M[r, ni + i] == 0 for i in range(len(Aeq))))
    # 新しい問題: 列 ν_0..ν_{ni-1} (整数・自由)、続けて連続列
    cols = [(f"v{k}", True, -INF, INF, float(sum(P["cost"][I[j]] * U[k][j] for j in range(ni)))) for k in range(ni)]
    cols += [(f"c{j}", False, P["cl"][j], P["cu"][j], P["cost"][j]) for j in C]
    cpos = {j: ni + t for t, j in enumerate(C)}
    rows, rl, ru = [], [], []
    for i in range(P["m"]):
        coef = {}
        for j, a in P["rows"][i]:
            if j in pos:
                for k in range(ni):
                    if U[k][pos[j]]: coef[k] = coef.get(k, 0.0) + a * U[k][pos[j]]
            else:
                coef[cpos[j]] = coef.get(cpos[j], 0.0) + a
        rows.append([(k, v) for k, v in sorted(coef.items()) if v != 0]); rl.append(P["rl"][i]); ru.append(P["ru"][i])
    for t, j in enumerate(I):  # 整数列の上下限
        if P["cl"][j] == -INF and P["cu"][j] == INF: continue
        rows.append([(k, float(U[k][t])) for k in range(ni) if U[k][t]]); rl.append(P["cl"][j]); ru.append(P["cu"][j])
    return cols, rows, rl, ru, ker, len(Aeq), max(abs(v) for r in U for v in r)

def run(mps, T):
    env = dict(os.environ, ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1")
    t0 = time.time()
    p = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(T), "--threads", "4"], capture_output=True, text=True, env=env)
    try: r = json.loads(p.stdout.strip().splitlines()[-1])
    except Exception: r = {"status": "error", "err": p.stderr[-300:]}
    return {k: r.get(k) for k in ("status", "objective", "bound", "time", "nodes", "err") if k in r}

if __name__ == "__main__":
    out = Path(sys.argv[1]); T = float(sys.argv[2])
    for name in sys.argv[3:]:
        P = read(name)
        t0 = time.time(); cols, rows, rl, ru, ker, neq, mx = reform(P); tl = time.time() - t0
        p1 = TMP / f"{name}_lat.mps"; write_mps(p1, cols, rows, rl, ru, P["off"])
        rec = {"problem": name, "nint": sum(1 for c in cols if c[1]), "neq": neq, "ker": ker, "maxU": mx, "lll_secs": round(tl, 3)}
        rec["orig"] = run(WT / ".miplib_cache" / f"{name}.mps", T)
        rec["lat"] = run(p1, T)
        with out.open("a") as f: f.write(json.dumps(rec) + "\n")
        print(json.dumps(rec), flush=True)

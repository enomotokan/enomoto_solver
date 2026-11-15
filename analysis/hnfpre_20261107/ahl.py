"""market split の実行可能性問題を、元の形と Aardal–Hurkens–Lenstra (2000) の格子の書き直し (x = x_d + Q λ、Q は LLL で
簡約した整数の核の基底) の形の両方で ENOMOTO に解かせて比べる (2026-11-07、HNFPRE の「置き換え」の効き目の見積もり)。
使い方: python ahl.py out.jsonl T [inst ...]。inst は "cd:m:seed" (Cornuéjols–Dawande 1999 の作り方: a_ij は 0〜99 の一様、
n = 10(m-1)、d_i = floor(Σ_j a_ij / 2)) か "mps:name" (MIPLIB の markshare の行から連続のスラックを除いたもの)。"""
import json, os, random, subprocess, sys, time
from pathlib import Path
from fpylll import IntegerMatrix, LLL
D = Path(__file__).resolve().parent
WT = D.parents[1]
TMP = Path(os.environ.get("AHL_TMP", "/tmp/ahl")); TMP.mkdir(exist_ok=True)

def gen_cd(m, seed):
    g = random.Random(seed); n = 10 * (m - 1)
    A = [[g.randrange(100) for _ in range(n)] for _ in range(m)]
    return A, [sum(r) // 2 for r in A]

def from_mps(name):
    import highspy, numpy as np
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(str(WT / ".miplib_cache" / f"{name}.mps"))
    lp = h.getLp(); n, m = lp.num_col_, lp.num_row_
    it = lp.integrality_; st = np.array(lp.a_matrix_.start_); idx = np.array(lp.a_matrix_.index_); val = np.array(lp.a_matrix_.value_)
    ints = [j for j in range(n) if int(it[j]) == 1]
    A = [[0] * len(ints) for _ in range(m)]
    for k, j in enumerate(ints):
        for p in range(st[j], st[j + 1]): A[idx[p]][k] = int(val[p])
    return A, [int(lp.row_lower_[i]) for i in range(m)]

def write_mps(path, lo, up, rows, rlo, rup):
    n = len(lo); L = ["NAME x", "ROWS", " N obj"]
    for i in range(len(rows)): L.append(f" {'E' if rlo[i] == rup[i] else 'G'} r{i}")
    L.append("COLUMNS"); L.append(" M0 'MARKER' 'INTORG'")
    cols = [[] for _ in range(n)]
    for i, r in enumerate(rows):
        for j, a in r: cols[j].append((i, a))
    for j in range(n):
        L.append(f" x{j} obj 0")
        for i, a in cols[j]: L.append(f" x{j} r{i} {a}")
    L.append(" M1 'MARKER' 'INTEND'"); L.append("RHS")
    for i in range(len(rows)): L.append(f" rhs r{i} {rlo[i]}")
    rng = [(i, rup[i] - rlo[i]) for i in range(len(rows)) if rlo[i] != rup[i] and rup[i] < float('inf')]
    if rng:
        L.append("RANGES"); L += [f" rng r{i} {v}" for i, v in rng]
    L.append("BOUNDS")
    for j in range(n):
        if lo[j] == -float('inf') and up[j] == float('inf'): L.append(f" FR bnd x{j}")
        else: L.append(f" LO bnd x{j} {lo[j]}"); L.append(f" UP bnd x{j} {up[j]}")
    L.append("ENDATA"); Path(path).write_text("\n".join(L) + "\n")

def ahl(A, d, N1=1000, N2=100000):
    """AHL の埋め込み [[I, 0], [0, N1], [N2 A, -N2 d]] の列を LLL で簡約し、(x_d, Q) を返す (Q の列は A の整数の核の基底)。"""
    m, n = len(A), len(A[0])
    B = []  # fpylll は行が基底のベクトル
    for j in range(n): B.append([1 if k == j else 0 for k in range(n)] + [0] + [N2 * A[i][j] for i in range(m)])
    B.append([0] * n + [N1] + [-N2 * d[i] for i in range(m)])
    M = IntegerMatrix.from_matrix(B); LLL.reduction(M)
    rows = [[M[i, k] for k in range(n + 1 + m)] for i in range(M.nrows)]
    Q = [r[:n] for r in rows if all(v == 0 for v in r[n:])]
    xd = [r for r in rows if r[n] != 0 and all(v == 0 for v in r[n + 1:])]
    if len(Q) != n - m or len(xd) != 1:
        return None
    x = xd[0]; s = -1 if x[n] > 0 else 1  # 列は (x, -N1, 0) なら A x = d (列の作り方 N1 * t に対し x の係数は... 符号を確かめる)
    xv = [s * v for v in x[:n]]
    if any(sum(A[i][j] * xv[j] for j in range(n)) != d[i] for i in range(m)):
        xv = [-v for v in xv]
    assert all(sum(A[i][j] * xv[j] for j in range(n)) == d[i] for i in range(m))
    return xv, Q

def run(mps, T):
    env = dict(os.environ, ENOMOTO_MIP_THREADS="4")
    t0 = time.time()
    p = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(T), "--threads", "4"], capture_output=True, text=True, env=env)
    try:
        r = json.loads(p.stdout.strip().splitlines()[-1])
    except Exception:
        r = {"status": "error", "err": p.stderr[-300:]}
    r["wall"] = round(time.time() - t0, 2)
    return {k: r.get(k) for k in ("status", "time", "nodes", "wall", "err") if k in r}

if __name__ == "__main__":
    out = Path(sys.argv[1]); T = float(sys.argv[2])
    for inst in sys.argv[3:]:
        if inst.startswith("cd:"):
            _, m, seed = inst.split(":"); A, d = gen_cd(int(m), int(seed))
        else:
            A, d = from_mps(inst.split(":", 1)[1])
        m, n = len(A), len(A[0])
        # 元の形: 0-1 の x、A x = d
        p0 = TMP / f"{inst.replace(':', '_')}_orig.mps"
        write_mps(p0, [0] * n, [1] * n, [[(j, A[i][j]) for j in range(n) if A[i][j]] for i in range(m)], d, d)
        t0 = time.time(); res = ahl(A, d); tl = time.time() - t0
        rec = {"inst": inst, "m": m, "n": n, "lll_secs": round(tl, 3)}
        rec["orig"] = run(p0, T)
        if res is None:
            rec["ahl"] = None
        else:
            xd, Q = res; k = len(Q)
            rows = [[(c, Q[c][j]) for c in range(k) if Q[c][j]] for j in range(n)]
            p1 = TMP / f"{inst.replace(':', '_')}_ahl.mps"
            write_mps(p1, [-float('inf')] * k, [float('inf')] * k, rows, [-xd[j] for j in range(n)], [1 - xd[j] for j in range(n)])
            rec["maxQ"] = max(abs(v) for r in Q for v in r)
            rec["ahl"] = run(p1, T)
        with out.open("a") as f: f.write(json.dumps(rec) + "\n")
        print(json.dumps(rec), flush=True)

"""大 M の行 (0-1 列 y が 1 なら同じ行の他の 0-1 列が全て 0、y = 0 なら行が冗長) を、P 58 問 + M (P と重ならない問題) の元の問題で数える。
行は 0-1 列だけ・3 列以上。負の係数は文字を補って (x -> 1 - x) 正にしてから判定する (BIGM_SPLIT の判定を補った文字に広げたもの)。
python bigmscan.py K NSHARD → bigmscan.jsonl_K (途中から再開できる)。求解はしない (highspy で読むだけ)"""
import json, sys
from pathlib import Path
import highspy, numpy as np
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
P = sorted({x[0] for s in json.loads((WT / "analysis/scipslow_20261119/shards_P.json").read_text())["shards"] for x in s})
M = [p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part]
probs = [("P", p) for p in P] + [("M", p) for p in M if p not in P]
k, ns = int(sys.argv[1]), int(sys.argv[2])
OUT = Path(str(D / "bigmscan.jsonl") + f"_{k}")
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()

def scan(mps):
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(str(mps))
    lp = h.getLp(); n, m = lp.num_col_, lp.num_row_
    lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
    integ = np.array(lp.integrality_) if len(lp.integrality_) else np.zeros(n)
    binc = (integ != 0) & (lo >= 0) & (up <= 1)
    a = lp.a_matrix_
    # 列ごとの行列 → 行ごと
    st, idx, val = np.array(a.start_), np.array(a.index_), np.array(a.value_)
    if a.format_ == highspy.MatrixFormat.kColwise:
        cols = np.repeat(np.arange(n), np.diff(st)); order = np.argsort(idx, kind="stable")
        ridx, rcol, rval = idx[order], cols[order], val[order]
        rst = np.searchsorted(ridx, np.arange(m + 1))
    else:
        rst, rcol, rval = st, idx, val
    rlo, rup = np.array(lp.row_lower_), np.array(lp.row_upper_)
    nrows, nnz, ylist = 0, 0, set()
    for i in range(m):
        cs, vs = rcol[rst[i]:rst[i + 1]], rval[rst[i]:rst[i + 1]]
        if len(cs) < 3 or not binc[cs].all():
            continue
        for sgn, b in ((1.0, rup[i]), (-1.0, -rlo[i])):
            if not np.isfinite(b) or abs(b) > 1e19:
                continue
            c = sgn * vs
            bb = b - c[c < 0].sum()  # 負の係数の文字を補う
            w = np.abs(c)
            k_ = int(np.argmax(w)); ay = w[k_]; rest = w.sum() - ay
            cmin = np.min(np.delete(w, k_))
            tol = 1e-9 * max(abs(bb), 1.0)
            if ay <= bb + tol and rest <= bb + tol and cmin > bb - ay + tol:
                nrows += 1; nnz += len(cs); ylist.add(int(cs[k_]))
                break
    return {"n": int(n), "m": int(m), "nnz": int(len(rval)), "bigm_rows": nrows, "bigm_nnz": nnz, "bigm_y": len(ylist)}

for s, p in probs[k::ns]:
    if p in done: continue
    row = {"set": s, "problem": p}
    try:
        had = (WT / ".miplib_cache" / f"{p}.mps").exists()
        mps = rb.fetch(p, WT / ".miplib_cache")
        try:
            row.update(scan(mps))
        finally:
            if not had: mps.unlink()  # 新しく取った問題は消す (ディスクの節約)
    except Exception as e:  # noqa: BLE001
        row["error"] = str(e)[:200]
    with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
    print(json.dumps(row), flush=True)

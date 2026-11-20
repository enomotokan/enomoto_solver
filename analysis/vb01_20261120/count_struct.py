"""VB01 が使える形かの構造の数え上げ (元の問題で。前処理の後とは違うので目安): python count_struct.py SET (P か M)。
0-1 列のうち衝突グラフ (行から作る組、当方の CliqueGraph::build と同じ規則、文字 2000 を超える行は使わない) に辺を持つ列の数 nb を数え、
VB01 の条件 (境界の節点 2 nb >= 0.65 × 整数列の数) を満たすかを count_struct_SET.jsonl に足す。MPS は数えた後に消す (KEEP=1 で残す)。"""
import json, os, sys
from pathlib import Path
import numpy as np
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
import highspy  # noqa: E402
SET = sys.argv[1]
if SET == "P":
    from probs import PROBS as probs  # noqa: E402
else:
    probs = sorted({x for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for x in part})
OUT = D / f"count_struct_{SET}.jsonl"
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines() if l.strip()} if OUT.exists() else set()
INF = 1e20
for p in probs:
    if p in done: continue
    cached = (WT / ".miplib_cache" / f"{p}.mps").exists()
    try:
        mps = rb.fetch(p, WT / ".miplib_cache")
        h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(str(mps)); lp = h.getLp()
    except Exception as e:  # noqa: BLE001
        with open(OUT, "a") as f: f.write(json.dumps({"problem": p, "error": str(e)[:200]}) + "\n")
        continue
    n, m = lp.num_col_, lp.num_row_
    lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
    integ = np.array([int(t) != 0 for t in lp.integrality_]) if len(lp.integrality_) else np.zeros(n, bool)
    binary = integ & (lo == 0) & (up == 1)
    A = lp.a_matrix_
    st, ix, va = np.array(A.start_), np.array(A.index_), np.array(A.value_)
    # 列ごとの CSC を行ごとに並べ替える
    cols = np.repeat(np.arange(n), np.diff(st))
    order = np.argsort(ix, kind="stable"); rr, cc, vv = ix[order], cols[order], va[order]
    rst = np.searchsorted(rr, np.arange(m + 1))
    rlo, rup = np.array(lp.row_lower_), np.array(lp.row_upper_)
    has = np.zeros(n, bool)
    for i in range(m):
        a, b = rst[i], rst[i + 1]
        if b - a < 2: continue
        J, V = cc[a:b], vv[a:b]
        for sign, rhs in ((1.0, rup[i]), (-1.0, -rlo[i])):
            if abs(rhs) >= INF: continue
            W = sign * V
            mn = np.where(W > 0, W * lo[J], W * up[J])
            if not np.all(np.isfinite(mn)) or np.any(np.abs(mn) >= INF): continue
            sel = binary[J] & (W != 0)
            if sel.sum() < 2 or sel.sum() > 2000: continue
            w = np.abs(W[sel]); js = J[sel]
            slack = rhs - mn.sum(); tol = 1e-9 * (1 + abs(rhs))
            o = np.argsort(-w); w1 = w[o[0]]; w2 = w[o[1]]
            if w1 + w2 <= slack + tol: continue
            # 文字 u に衝突があるのは、u 以外の最大の重みと足して余裕を超えるとき
            other = np.where(np.arange(len(w)) == o[0], w2, w1)
            has[js[w + other > slack + tol]] = True
    nint = int(integ.sum()); nb = int(has.sum())
    r = {"problem": p, "n": n, "m": m, "nint": nint, "nbin": int(binary.sum()), "nb_conflict": nb, "cand": bool(nint > 0 and 2 * nb >= 0.65 * nint)}
    with open(OUT, "a") as f: f.write(json.dumps(r) + "\n")
    print(json.dumps(r), flush=True)
    if not cached and not os.environ.get("KEEP"):
        mps.unlink(missing_ok=True)

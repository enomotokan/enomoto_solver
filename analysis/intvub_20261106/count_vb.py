"""一般整数列 w (上下限の幅 > 1.5) と 0-1 列 z の 2 項の行 (変数上下限 w <= u z / w >= l z の元) を、元の模型で数える (2026-11-06)。
連続列と整数列の 2 項の行 (今の VarBounds::from_rows が集めるもの) も数える。問題は 58 問 (analysis/cbcimpl_20261029/probs.py) と
Mittelmann 186 問 (analysis/cbcimpl_20261029/mitt_parts.json)。count_vb.jsonl に 1 問 1 行。途中から再開できる。
使い方: python count_vb.py [58|mitt] (大きな Mittelmann の問題は数えた後で消す)"""
import json, os, sys, collections
from pathlib import Path
import numpy as np
import highspy
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
which = sys.argv[1] if len(sys.argv) > 1 else "58"
probs = PROBS if which == "58" else sorted({p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part})
OUT = D / "count_vb.jsonl"
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
for p in probs:
    if p in done:
        continue
    try:
        mps = rb.fetch(p, WT / ".miplib_cache")
        h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(str(mps))
        lp = h.getLp(); n, m = lp.num_col_, lp.num_row_
        lo, up = np.array(lp.col_lower_), np.array(lp.col_upper_)
        integ = np.array([int(t) for t in lp.integrality_]) if len(lp.integrality_) else np.zeros(n, int)
        isint = integ == 1
        isbin = isint & (lo == 0) & (up == 1)
        isgen = isint & (up - lo > 1.5)
        A = lp.a_matrix_
        st, ix = np.array(A.start_), np.array(A.index_)
        if A.format_ == highspy.MatrixFormat.kColwise:
            rowcnt = np.bincount(ix, minlength=m)
            colof = np.repeat(np.arange(n), np.diff(st))
            rowof = ix
        else:
            rowcnt = np.diff(st); rowof = np.repeat(np.arange(m), np.diff(st)); colof = ix
        two = rowcnt[rowof] == 2
        r2, c2 = rowof[two], colof[two]
        order = np.argsort(r2, kind="stable"); r2, c2 = r2[order], c2[order]
        a, b = c2[0::2], c2[1::2]
        gb = int(np.sum((isgen[a] & isbin[b]) | (isgen[b] & isbin[a])))
        ci = int(np.sum(((~isint[a]) & isint[b]) | ((~isint[b]) & isint[a])))
        gcols = set(a[isgen[a] & isbin[b]].tolist()) | set(b[isgen[b] & isbin[a]].tolist())
        rec = {"problem": p, "set": which, "n": n, "m": m, "ngen": int(isgen.sum()), "nbin": int(isbin.sum()), "ncont": int((~isint).sum()),
               "rows_genint_bin": gb, "genint_cols_with_vb": len(gcols), "rows_cont_int": ci}
    except Exception as e:  # noqa: BLE001
        rec = {"problem": p, "set": which, "error": str(e)[:200]}
    with open(OUT, "a") as f:
        f.write(json.dumps(rec) + "\n")
    print(json.dumps(rec), flush=True)
    if which == "mitt":
        try:
            if mps.stat().st_size > 50e6: os.remove(mps)
        except Exception:  # noqa: BLE001
            pass

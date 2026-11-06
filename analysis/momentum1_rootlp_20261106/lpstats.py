"""書き出した LP (MPS、gz 可) の大きさ・係数の幅・幅の狭い行と列の数を出す。使い方: python lpstats.py MPS..."""
import gzip, shutil, sys, tempfile
import numpy as np, highspy
for f in sys.argv[1:]:
    src = f
    if f.endswith(".gz"):
        tmp = tempfile.NamedTemporaryFile(suffix=".mps", delete=False); shutil.copyfileobj(gzip.open(f), tmp); tmp.close(); src = tmp.name
    h = highspy.Highs(); h.setOptionValue("output_flag", False); h.readModel(src); lp = h.getLp()
    a = np.abs(np.array(lp.a_matrix_.value_)); a = a[a > 0]
    rl, ru = np.array(lp.row_lower_), np.array(lp.row_upper_); cl, cu = np.array(lp.col_lower_), np.array(lp.col_upper_)
    rw, cw = ru - rl, cu - cl
    print(f, {"m": lp.num_row_, "n": lp.num_col_, "nnz": len(a), "amin": float(a.min()), "amax": float(a.max()), "range": float(a.max() / a.min()),
              "coef<1e-4": int((a < 1e-4).sum()), "rows 0<w<1e-6": int(((rw > 0) & (rw < 1e-6)).sum()), "rows eq": int((rw == 0).sum()),
              "cols 0<w<1e-6": int(((cw > 0) & (cw < 1e-6)).sum()), "cols fixed": int((cw == 0).sum())})

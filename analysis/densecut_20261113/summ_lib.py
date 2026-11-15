"""summ.py・diag.py の共通: DENSECUT 行の集計 (スレッドの和)。"""
import re
from collections import defaultdict
def num(s, key):
    m = re.search(re.escape(key) + r" ([-0-9.e]+)", s); return float(m.group(1)) if m else 0.0
def agg(r):
    a = defaultdict(float); hist = [0] * 5
    for s in r["dc"]:
        lps = num(s, " lps"); a["lps"] += lps; a["nodes"] += num(s, " nodes")
        a["rows"] += num(s, "rows/lp") * lps; a["post"] += num(s, "post rows/lp") * lps; a["postnz"] += num(s, "post nnz/lp") * lps
        a["lpsecs"] += num(s, "lpsecs"); a["cutsecs"] += num(s, "cutsecs")
        nc = num(s, "ncuts"); a["ncuts"] += nc; a["nnz"] += num(s, "nnz/cut") * nc
        a["rowavg"] = num(s, "rowavg"); a["m"] = num(s, " m"); a["orignnz"] = num(s, "orignnz")
        gn = num(s, "gain n"); a["gn"] += gn; a["gsum"] += num(s, "mean") * gn
        a["dd"] += num(s, "dense depth"); a["dh"] += num(s, "dense hits")
        h = re.search(r"hist \[([0-9, ]+)\]", s)
        if h: hist = [x + int(y) for x, y in zip(hist, h.group(1).split(","))]
    a["hist"] = hist
    return a

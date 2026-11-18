"""問題の簡単な特徴 (MPS から: 行・列・非零・整数列の割合・2 値列の割合・連続列の割合) を出す (2026-11-18)。
使い方: python feat.py 問題名 ... (リポジトリの .miplib_cache/<問題>.mps を読む)"""
import sys, json
from pathlib import Path
C = Path(__file__).resolve().parents[2] / ".miplib_cache"
for p in sys.argv[1:]:
    f = C / f"{p}.mps"
    if not f.exists():
        print(json.dumps({"problem": p, "missing": True})); continue
    sec = None; rows = 0; cols = {}; nnz = 0; intm = False; ints = set(); bnd = {}
    for l in open(f, errors="ignore"):
        if not l.strip() or l.startswith("*"): continue
        if not l[0].isspace():
            sec = l.split()[0]; continue
        t = l.split()
        if sec == "ROWS":
            if t[0] != "N": rows += 1
        elif sec == "COLUMNS":
            if "'MARKER'" in t:
                intm = "'INTORG'" in t; continue
            c = t[0]; cols.setdefault(c, None)
            if intm: ints.add(c)
            nnz += sum(1 for i in range(1, len(t) - 1, 2) if True)
        elif sec == "BOUNDS":
            if t[0] == "BV": ints.add(t[2]); bnd[t[2]] = (0, 1)
            elif t[0] in ("UP", "LO") and len(t) > 3:
                lo, up = bnd.get(t[2], (0, None))
                bnd[t[2]] = (float(t[3]), up) if t[0] == "LO" else (lo, float(t[3]))
    n = len(cols); nb = sum(1 for c in ints if bnd.get(c, (0, None)) == (0, 1.0) or bnd.get(c) == (0, 1))
    print(json.dumps({"problem": p, "rows": rows, "cols": n, "nnz": nnz, "int_frac": round(len(ints) / max(n, 1), 3),
                      "bin_frac": round(nb / max(n, 1), 3), "cont_frac": round(1 - len(ints) / max(n, 1), 3)}))

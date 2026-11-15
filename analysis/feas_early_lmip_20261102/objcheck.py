"""MIPLIB 2017 の問題を取得し、MPS の目的関数の行 (最初の N 行) に 0 でない係数があるかを数える (2026-11-02)。
使い方: python objcheck.py 問題名...  (.miplib_cache に取得する)"""
import sys
from pathlib import Path
sys.path.insert(0, "scripts")
import run_miplib_benchmark as rb
for p in sys.argv[1:]:
    try:
        mps = rb.fetch(p, Path(".miplib_cache"))
    except Exception as e:
        print(p, "fetch failed", e, flush=True); continue
    obj = None; sec = None; nz = 0; ncols = set(); rows = 0
    for line in open(mps):
        if not line.strip() or line.startswith("*"): continue
        if not line[0].isspace():
            sec = line.split()[0]; continue
        t = line.split()
        if sec == "ROWS":
            if t[0] == "N" and obj is None: obj = t[1]
            else: rows += 1
        elif sec == "COLUMNS":
            if "MARKER" in line: continue
            ncols.add(t[0])
            for k in range(1, len(t) - 1, 2):
                if t[k] == obj and float(t[k + 1]) != 0.0: nz += 1
        elif sec in ("RHS", "BOUNDS", "RANGES"): break
    print(f"{p} rows {rows} cols {len(ncols)} objnz {nz}", flush=True)

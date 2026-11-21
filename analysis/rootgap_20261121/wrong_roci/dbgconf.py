"""「conflict (lits) cuts off the debug solution」の行ごとに、デバッグ解が各リテラルをどれだけ外れるか (正 = 厳密には満たさない) の最大を出す。
python dbgconf.py ログ.gz ...  各行の最大の外れが 0 より大きければ、そのリテラルは厳密には成り立たず、診断の許容 1e-6 による見かけの出力"""
import gzip, re, sys
pl = re.compile(r"x(\d+)(<=|>=)([-0-9.e+inf]+)\[([-0-9.e+inf]+)\]")
for fn in sys.argv[1:]:
    gaps = []
    for l in gzip.open(fn, "rt"):
        if "conflict (lits) cuts off the debug solution" not in l: continue
        st = l.split("stack")[1].split(":", 1)[1]
        g = max((float(x) - float(v)) if op == "<=" else (float(v) - float(x)) for j, op, v, x in pl.findall(st))
        gaps.append(g)
    strict = sum(1 for g in gaps if g <= 0)
    print(fn, "lines", len(gaps), "strictly satisfied (real cutoff)", strict, "max gap", max(gaps, default=None), "min gap", min(gaps, default=None))

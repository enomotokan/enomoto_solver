"""HIGHSLOSS2: logs/hsep_*.log.gz (改造 HiGHS の HSEP 行) を 1 行ずつにまとめる。python hsum.py [問題の接頭辞]
出力: 問題 切った分離器 根のラウンド数 根の最初の LP 値 -> 最後のラウンドの後の LP 値、分離器ごとにプールに入った本数の和、LP に入った本数の和、
始めの伝播で固定された列の数の最大"""
import gzip, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
pre = sys.argv[1] if len(sys.argv) > 1 else ""
for f in sorted((D / "logs").glob(f"hsep_{pre}*.log.gz")):
    rs = []
    fx = 0
    for l in gzip.open(f, "rt"):
        m = re.match(r"HSEP round: obj (\S+) -> (\S+) pool\+ impl (\d+) clq (\d+) tab (\d+) path (\d+) modk (\d+); to LP (\d+)", l)
        if m:
            rs.append([float(m[1]), float(m[2])] + [int(m[k]) for k in range(3, 9)])
        m = re.match(r"HSEP step propagate: \d+ bound changes, obj \S+ fixed global (\d+)", l)
        if m:
            fx = max(fx, int(m[1]))
    if not rs:
        print(f.name, "no rounds"); continue
    s = [sum(r[k] for r in rs) for k in range(2, 8)]
    print(f"{f.name[5:-7]:45s} rounds {len(rs):3d} LP {rs[0][0]:.4f} -> {rs[-1][1]:.4f}  pool impl {s[0]} clq {s[1]} tab {s[2]} path {s[3]} modk {s[4]} toLP {s[5]} fixed {fx}")

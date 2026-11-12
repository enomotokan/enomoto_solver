"""RCSTRONG2 の集計: jsonl ごとに 問題 x 設定 x 種 の時間 (解けなければ * と下界・暫定解)、ノードの LP/ノード、平均の行、ノードで足したカット。
使い方: python summ.py [jsonl ...] (なければ全部)"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
files = [Path(a) for a in sys.argv[1:]] or sorted(D.glob("*.jsonl"))
for f in files:
    f = f if f.is_absolute() else D / f
    print(f"## {f.name}")
    rows = {}
    for l in f.read_text().splitlines():
        if not l.strip():
            continue
        r = json.loads(l)
        res = r["result"]
        t = res.get("time")
        st = f"{t:6.1f}" if res.get("status") == "optimal" else f"{r['tl']}*(lb {res.get('bound', float('nan')):.6g} ub {res.get('objective', float('nan')):.6g})"
        nd = [d for d in r.get("diag", []) if "MIP: node diag:" in d]
        extra = ""
        if nd:
            m = re.search(r"nodes (\d+) node LPs (\d+) mean rows ([\d.]+).*added (\d+)", nd[-1])
            if m:
                n, lps, rows_, add = int(m[1]), int(m[2]), float(m[3]), int(m[4])
                extra = f" LP/node {lps / max(n, 1):.2f} rows {rows_:.0f} cuts {add}"
        rows.setdefault(r["problem"], []).append((r["cfg"], r["seed"], st + extra))
    for p, v in rows.items():
        print(p)
        for c, s, x in sorted(v):
            print(f"  {c:6s} s{s} {x}")

"""使い方: python side_table.py   runs4/side_<問題>_<設定>_<回>.log を表にする (時間・状態・上界・下界・ノード・根の下界)"""
import re, glob, collections
rows = collections.defaultdict(dict)
for f in sorted(glob.glob("runs4/side/side_*.log")):
    m = re.match(r"runs4/side/side_(.+)_([a-z0-9]+)_([ab])\.log", f)
    prob, cfg, rep = m.groups()
    s = open(f).read()
    r = re.search(r"RESULT ([\d.]+) (\{.*\})", s)
    if not r:
        rows[prob][(cfg, rep)] = "running"; continue
    d = eval(r.group(2).replace("inf", "float('inf')").replace("nan", "float('nan')"))
    st = "OPT" if d["status"] == "optimal" else d["status"][:4]
    rows[prob][(cfg, rep)] = f"{st} {float(r.group(1)):.1f}s ub {d['objective']:.6g} lb {d['best_bound']:.6g} n {d['nodes']} root {d['root_bound']:.6g}"
for prob, v in rows.items():
    for k in sorted(v):
        print(f"{prob:18s} {k[0]:7s} {k[1]}  {v[k]}")

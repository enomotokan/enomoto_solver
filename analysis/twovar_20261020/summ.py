"""cmp.sh のログを問題・設定ごとにまとめる: summ.py <ログのディレクトリ> [問題 ...]"""
import ast, glob, os, re, sys
from collections import defaultdict
rows = defaultdict(list)
only = set(sys.argv[2:])
for f in sorted(glob.glob(os.path.join(sys.argv[1], "*.log"))):
    m = re.match(r"(.+)_([a-z0-9]+)_(\d+)\.log", os.path.basename(f))
    if not m:
        continue
    p, c, r = m.groups()
    if only and p not in only:
        continue
    txt = open(f).read()
    pre = re.search(r"^Presolve: .*-> (\d+) rows, (\d+) columns \(([\d.]+)s\)", txt, re.M)
    res = re.search(r"^RESULT (\{.*\}) ([\d.]+)", txt, re.M)
    lp = re.search(r"^Root LP: objective ([-\d.e+]+) \(([\d.]+)s\)", txt, re.M)
    if not res:
        rows[(p, c)].append("結果なし")
        continue
    d = eval(res.group(1), {"inf": float("inf"), "nan": float("nan")})
    t = float(res.group(2))
    st = "最適" if d["status"] == "optimal" else "時間切れ"
    obj = d["objective"]
    fs = d["first_solution_time"]
    s = f"{st} {t:.1f}s 値 {obj if obj is None else round(obj, 2)} 下界 {round(d['best_bound'], 2)} {d['nodes']}N 根 {round(d['root_bound'], 1)}"
    if lp:
        s += f" (LP {float(lp.group(1)):.1f} @{lp.group(2)}s)"
    if pre:
        s += f" 前処理 {pre.group(1)}x{pre.group(2)} {pre.group(3)}s"
    if fs is not None:
        s += f" 初解 {round(d['first_solution_objective'], 1)}@{fs:.1f}s"
    rows[(p, c)].append(s)
for (p, c), v in sorted(rows.items()):
    print(f"{p} {c}:\n  " + "\n  ".join(v))

"""使い方: python summ.py <ログ>...   ENOMOTO / HiGHS のログから要点を 1 行ずつ出す"""
import re, sys
for f in sys.argv[1:]:
    s = open(f).read()
    if "HiGHS" in s[:2000] or "Running HiGHS" in s:
        pre = re.search(r"Presolved problem has (\d+) rows; (\d+) cols", s) or re.search(r"Reduced problem has? *(\d+) rows", s)
        m = re.search(r"RESULT (\S+) (\S+) (\S+) (\S+)", s)
        sol = re.findall(r"^ ([A-Z ]) +\d+ +\d+ +\d+ +[\d.]+% +([\d.e+-]+) +([\d.e+-]+) .* ([\d.]+)s$", s, re.M)
        print(f, "HiGHS", m.groups() if m else None)
        continue
    d = {}
    m = re.search(r"RESULT [\d.]+ (\{.*\})", s)
    if m:
        d = eval(m.group(1).replace("inf", "float('inf')").replace("nan", "float('nan')"))
        d.pop("x", None)
    pre = re.search(r"Presolve: .* -> (\d+) rows, (\d+) columns", s)
    rc = re.search(r"Root cuts: .*-> (\S+), (\d+) cuts in the LP \(([\d.]+)s\)", s)
    bg = re.search(r"background cuts: (\d+) rounds.*-> (\S+) \(.*, ([\d.]+)s\)", s)
    rounds = len(re.findall(r"cut round \d+", s))
    inc = re.findall(r"new incumbent (\S+) at node (\d+) \(([\d.]+)s\) \[([^\]]+)\]", s)
    print(f"{f}: pre {pre.groups() if pre else None} rootcut {rc.groups() if rc else None} rounds {rounds} bg {bg.groups() if bg else None}")
    print("   inc", [(round(float(a)), b, t, h[:14]) for a, b, t, h in inc])
    print("   ", {k: (round(v, 1) if isinstance(v, float) else v) for k, v in d.items() if k in ("status", "objective", "best_bound", "nodes", "root_bound", "first_solution_time", "first_solution_objective")})

"""logs/<問題>_<設定>_<n>.log の最後の JSON を表にする: python summ.py [問題 ...]"""
import glob, json, re, sys, collections
rows = collections.defaultdict(list)
for f in sorted(glob.glob("logs/*.log")):
    m = re.match(r"logs/([^_]+)_(.+)_(\d+)\.log$", f)
    if not m:
        continue
    try:
        last = [l for l in open(f) if l.startswith("{")][-1]
    except IndexError:
        continue
    r = json.loads(last)
    rows[(m.group(1), m.group(2))].append(r)
probs = sys.argv[1:]
for (p, c), rs in sorted(rows.items()):
    if probs and p not in probs:
        continue
    s = "; ".join(f"{r['status'][:4]} {r['objective'] if r['objective'] is None else round(r['objective'], 2)} "
                  f"bd {round(r['best_bound'], 2) if r['best_bound'] not in (None,) and r['best_bound'] > -1e300 else '-'} "
                  f"{r['time']:.1f}s 1st {None if r['first_solution_time'] is None else round(r['first_solution_time'], 1)}s" for r in rs)
    print(f"{p:20s} {c:12s} {s}")

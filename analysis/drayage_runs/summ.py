# side.sh の結果 (TSV) を 1 行ずつまとめる
import ast, sys
for l in open(sys.argv[1]):
    r, p, c, res = l.rstrip('\n').split('\t', 3)
    if not res:
        print(r, p, c, 'NO RESULT'); continue
    t = res.split(' ', 2); d = eval(t[2], {'inf': float('inf'), 'nan': float('nan'), 'None': None})
    print(f"{r} {p:15s} {c:10s} t={t[1]:>6} {d['status']:10s} obj={d['objective'] if d['objective'] is None else round(d['objective'],4)} bd={d['best_bound']:.8g} nodes={d['nodes']} root={d['root_bound']:.8g} first={d['first_solution_time']}/{d['first_solution_objective']}")

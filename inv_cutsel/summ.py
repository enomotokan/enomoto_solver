"""ログの要点: 根の下界 (Root cuts の最後の値と各回)、再スタート数、木に入った時刻 (最初の経過の行)、最終の下界・暫定解・時間。"""
import re, sys, json
for fn in sys.argv[1:]:
    roots, restarts, tree_t, res = [], 0, None, None
    for l in open(fn):
        if 'Root cuts:' in l:
            m = re.search(r'-> ([-\d.e+]+)', l); roots.append(float(m.group(1)))
        elif 'Restart ' in l and 'newly fixed' in l:
            restarts += 1
        elif tree_t is None and re.match(r'\[\s*[\d.]+\]\s+[\d.]+s\s+\d+', l):
            tree_t = float(re.search(r'\[\s*([\d.]+)\]', l).group(1))
        elif l.split('] ', 1)[-1].startswith('RESULT '):
            res = json.loads(l.split('RESULT ', 1)[1])
    name = fn.split('/')[-1][:-4]
    if res is None:
        print(f'{name:45s} (no result)'); continue
    st = 'opt' if res['status'] == 'optimal' else res['status']
    rs = '/'.join(f'{r:.6g}' for r in roots)
    print(f"{name:45s} roots {rs:40s} rst {restarts} tree {tree_t if tree_t is not None else '-'!s:>6} {st:10s} t {res['time']:6.1f} ub {res['objective']} lb {res['bound']:.6g} nodes {res['nodes']}")

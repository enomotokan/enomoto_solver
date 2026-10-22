# logs/side/ の既定 (base) と試作 (kpnf) を並べる: 時間 (時間切れは TL と下界・暫定解)、ノード
import glob, re, os, sys
d = sys.argv[1] if len(sys.argv) > 1 else 'analysis/tree_replay_20261022/logs/side'
rows = {}
for f in glob.glob(d + '/*.out'):
    b = os.path.basename(f)[:-4]; p, cfg = b.rsplit('_', 1)
    m = re.search(r'RESULT t=([\d.]+) status=(\S+) obj=(\S+) bound=(\S+) nodes=(\d+)', open(f).read())
    if m: rows.setdefault(p, {})[cfg] = m.groups()
def fmt(r):
    if not r: return '-'
    t, st, ob, bd, n = r
    return f"{float(t):.1f} ({n})" if st == 'optimal' else f"{st} obj {ob} lb {float(bd):.6g} ({n})"
solved = {'base': 0, 'kpnf': 0}
print('| 問題 | 既定 | kpnf |'); print('|---|---|---|')
for p in sorted(rows):
    r = rows[p]
    for c in solved: solved[c] += r.get(c, ('', ''))[1] == 'optimal'
    print(f"| {p} | {fmt(r.get('base'))} | {fmt(r.get('kpnf'))} |")
print('solved', solved)

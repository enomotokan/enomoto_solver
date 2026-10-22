# logs/<dir>/<問題>_<設定><回>.out を問題 × 設定で並べる (時間、TL は時間切れ、解けた回数)
import glob, re, os, sys, collections
d = sys.argv[1]
R = collections.defaultdict(lambda: collections.defaultdict(list))
for f in sorted(glob.glob(d + '/*.out')):
    b = os.path.basename(f)[:-4]
    p, rest = b.rsplit('_', 1)
    cfg = rest.rstrip('0123456789')
    m = re.search(r't=([\d.]+) status=(\S+)', open(f).read())
    if m: R[p][cfg].append('%.1f' % float(m.group(1)) if m.group(2) == 'optimal' else 'TL')
cfgs = sorted({c for p in R for c in R[p]}, key=lambda c: (c != 'base', c))
print('| 問題 | ' + ' | '.join(cfgs) + ' |'); print('|---' * (len(cfgs) + 1) + '|')
for p in sorted(R):
    print(f"| {p} | " + ' | '.join(', '.join(R[p][c]) + f" ({sum(x != 'TL' for x in R[p][c])}/{len(R[p][c])})" for c in cfgs) + ' |')

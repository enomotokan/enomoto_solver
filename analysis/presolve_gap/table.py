# logs/q_*.txt の行を 問題 x 設定 の表にする (秒 or TL 暫定解、CHECK、警告)
import sys, re, collections
res = collections.defaultdict(lambda: collections.defaultdict(list))
for f in sys.argv[1:]:
    for line in open(f):
        m = re.match(r'(\S+) (\S+)_(\d+): .*RESULT (\S+) (\S+) (\S+) (\d+) ([\d.]+)', line)
        if not m: continue
        p, c, _, st, obj, bd, nodes, t = m.groups()
        chk = 'BAD' if 'CHECK BAD' in line else ''
        w = re.search(r'warn (\d+)', line); w = int(w.group(1)) if w else 0
        s = f"{float(t):.1f}" if st == 'optimal' else f"TL {obj}"
        if chk: s += ' BAD'
        if w: s += f' W{w}'
        res[p][c].append(s)
cs = sorted({c for p in res for c in res[p]}, key=lambda c: (c != 'def', c))
print('| 問題 | ' + ' | '.join(cs) + ' |'); print('|---' * (len(cs) + 1) + '|')
for p in res:
    print(f'| {p} | ' + ' | '.join(' / '.join(res[p][c]) for c in cs) + ' |')

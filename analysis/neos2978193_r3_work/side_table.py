"""logs/side_<問題>_<設定>_<n>.log を表にする (実時間 / ノード、時間切れは gap)。"""
import json, collections, glob, re
d = collections.defaultdict(list)
for f in sorted(glob.glob('logs/side_*.log')):
    m = re.match(r'logs/side_(.+)_(def|plq1|pq1s|sq1|q05)_(\d)\.log', f)
    r = json.loads([l for l in open(f) if l.startswith('{')][-1])
    ob, bb = r['objective'], r['best_bound']
    if r['status'] == 'optimal':
        s = f"{r['time']:.1f}"
    elif ob is None:
        s = "TL 解なし"
    else:
        s = f"TL gap {abs(ob - bb) / max(abs(ob), 1e-9) * 100:.2f}%"
    d[(m[1], m[2])].append(f"{s} / {r['nodes']}")
cfgs = ['def', 'plq1', 'pq1s', 'sq1', 'q05']
for p in dict.fromkeys(k[0] for k in d):
    print(f"| {p} | " + ' | '.join(', '.join(d[(p, c)]) for c in cfgs) + ' |')

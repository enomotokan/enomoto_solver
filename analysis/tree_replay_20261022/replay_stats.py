# ENOMOTO のなぞり (ER 行) と HiGHS のなぞりファイルを突き合わせる
# 使い方: python3 replay_stats.py <replay.txt> <ENOMOTO の標準エラー>
import sys, collections
H = {}
for l in open(sys.argv[1]):
    t = l.split()
    rs = [x for x in t[7:] if x.startswith('#')]
    key = tuple(sorted((p[:p.index('u') if 'u' in p else p.index('d')], 'u' in p) for p in t[7:] if not p.startswith('#')))
    H[key] = dict(seq=int(t[0]), res=int(t[1]), col=int(t[2]), lp=t[4], nfix=int(t[5]), depth=int(t[6]), rs={x[0]: int(x[1:]) for x in rs[0][1:].split(',')} if rs else {})
E = []
for l in open(sys.argv[2]):
    if not l.startswith('ER '):
        continue
    t = l.split()
    path = [x for x in t[13:] if not x.startswith('#')]
    rs = [x for x in t[13:] if x.startswith('#')]
    key = tuple(sorted((p[:p.index('u') if 'u' in p else p.index('d')], 'u' in p) for p in path))
    E.append(dict(rs={x[0]: int(x[1:]) for x in rs[0][1:].split(',')} if rs else {}, no=int(t[1]), seq=int(t[2]), depth=int(t[3]), out=t[4], obj0=float(t[5]), obj=float(t[6]), col=int(t[7]), nfix=int(t[8]), rows=int(t[9]), hres=int(t[10]), hlp=float(t[11]), hnfix=int(t[12]), key=key))
seen = {e['key'] for e in E}
inH = [e for e in E if e['seq'] >= 0]
ext = [e for e in E if e['seq'] < 0]
print(f"ENOMOTO nodes {len(E)}: in HiGHS file {len(inH)}, not in file {len(ext)}; HiGHS file nodes {len(H)}, of which reached by ENOMOTO {sum(1 for k in H if k in seen)}")
hl = lambda r: 'leaf' if r in (0, 1, 2, 4) else 'branch'
eo = lambda o: 'branch' if o == 'br' else 'leaf'
tab = collections.Counter((hl(e['hres']), e['out']) for e in inH)
print("HiGHS outcome x ENOMOTO outcome (nodes in the file):")
for k, v in sorted(tab.items()):
    print(f"  HiGHS {k[0]:6s} ENOMOTO {k[1]:8s} {v}")
# HiGHS leaf where ENOMOTO branched: size of the extra subtree
par = {}
for e in ext:
    # nearest ancestor in the file
    k = list(e['key'])
    anc = None
    # remove the last-added branching (unknown order): search the subsets by removing elements greedily
    best = None
    for kk in H:
        pass
    par[e['no']] = None
# extra nodes: classify by the deepest ancestor in the file (by prefix of the path in ENOMOTO's order)
cls = collections.Counter()
roots = collections.Counter()
hres_of_root = collections.Counter()
for l in open(sys.argv[2]):
    if not l.startswith('ER '):
        continue
    t = l.split()
    if int(t[2]) >= 0:
        continue
    path = [x for x in t[13:] if not x.startswith('#')]
    a = None
    for i in range(len(path), -1, -1):
        key = tuple(sorted((p[:p.index('u') if 'u' in p else p.index('d')], 'u' in p) for p in path[:i]))
        if key in H:
            a = (i, key)
            break
    if a is None:
        cls['no ancestor'] += 1
        continue
    i, key = a
    h = H[key]
    if i == len(path) - 1 and hl(h['res']) == 'branch':
        cls['child of a HiGHS-branched node not evaluated by HiGHS'] += 1
    else:
        cls[f'below a HiGHS leaf (res {h["res"]})' if hl(h['res']) == 'leaf' else 'below a HiGHS branch (deeper)'] += 1
        roots[key] += 1
print("nodes not in the file:")
for k, v in cls.most_common():
    print(f"  {k}: {v}")
print(f"  distinct HiGHS leaves with an ENOMOTO subtree below: {len(roots)}; subtree sizes (top): {sorted(roots.values(), reverse=True)[:15]}")
# LP value at the same node, for nodes both evaluated with an LP
d = [(e['obj0'], e['obj'], e['hlp']) for e in inH if e['hlp'] == e['hlp'] and abs(e['hlp']) != float('inf') and e['obj'] == e['obj']]
if d:
    import statistics
    diffs = [o - h for o0, o, h in d if abs(o) != float('inf')]
    print(f"LP at the same node (both have a finite LP): n={len(diffs)}, ENOMOTO - HiGHS: mean {statistics.mean(diffs):.4g}, median {statistics.median(diffs):.4g}, <-1e-6: {sum(x < -1e-6 for x in diffs)}, >1e-6: {sum(x > 1e-6 for x in diffs)}")
nf = [(e['nfix'], e['hnfix']) for e in inH if e['hnfix'] >= 0]
if nf:
    print(f"fixed integer columns at the node: ENOMOTO - HiGHS: mean {sum(a - b for a, b in nf) / len(nf):.3f}, ENOMOTO more {sum(a > b for a, b in nf)}, fewer {sum(a < b for a, b in nf)}")

# fixing reasons at the same node (only nodes where ENOMOTO solved the LP or branched, i.e. after full propagation)
import collections
er = collections.Counter(); hr = collections.Counter(); k = 0
for e in inH:
    if e['out'] not in ('br', 'lp') or not e['rs']:
        continue
    h = H[e['key']]
    er.update(e['rs']); hr.update(h['rs']); k += 1
if k:
    print(f"bound changes on integer columns per node at the same nodes (n={k}, ENOMOTO nodes that reached the LP):")
    print("  ENOMOTO:", {x: round(v / k, 2) for x, v in er.items()})
    print("  HiGHS:  ", {x: round(v / k, 2) for x, v in hr.items()})

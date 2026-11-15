import sys, gzip, statistics as st, collections as C
hp = {}
for l in open('/tmp/claude-0/rdump/hp_mc2.err', errors='replace'):
    if not l.startswith('HP '): continue
    f = l.split()
    tag = [x for x in f if x.startswith('#B')]
    cnt = dict((kv[0], int(kv[1:])) for kv in tag[0][1:].split(',')) if tag else {}
    hp[int(f[1])] = dict(depth=int(f[2]), res=int(f[3]), lp=float(f[4]), lb=float(f[5]), rows=int(f[6]), nfix=int(f[8]), cnt=cnt)
er = []
for l in open(sys.argv[1], errors='replace'):
    if not l.startswith('ER '): continue
    f = l.split()
    tag = [x for x in f if x.startswith('#B')]
    cnt = dict((kv[0], int(kv[1:])) for kv in tag[0][1:].split(',')) if tag else {}
    er.append(dict(seq=int(f[2]), depth=int(f[3]), out=f[4], obj0=float(f[5]), obj=float(f[6]), nfix=int(f[8]), cnt=cnt))
m = [(e, hp[e['seq']]) for e in er if e['seq'] >= 0 and e['seq'] in hp]
print('ER', len(er), 'matched', len(m))
off = st.median([h['lp'] - e['obj'] for e, h in m if e['depth'] == 0 and e['obj'] == e['obj']])
CUT = 211913.0001
print('offset', off)
tab = C.Counter((e['out'], h['res']) for e, h in m)
print(sorted(tab.items(), key=lambda x: -x[1]))
# HiGHS が LP 値で刈った (res 0) のにこちらが分枝したノード
print('-- HiGHS res0, ours br: depth, ours obj gap to cutoff, HiGHS lp gap, ours R/Q, HiGHS R+Q, ours nfix, HiGHS nfix')
rows = []
for e, h in m:
    if h['res'] == 0 and e['out'] == 'br':
        ho = h['lp'] - off
        rows.append((e['depth'], CUT - e['obj'], CUT - ho, e['cnt'].get('R', 0), h['cnt'].get('R', 0) + h['cnt'].get('Q', 0), e['nfix'], h['nfix'], sum(e['cnt'].values()), sum(h['cnt'].values())))
for r in sorted(rows): print('%2d ours %8.1f highs %8.1f | R %4d vs %4d | nfix %4d vs %4d | stack %4d vs %4d' % r)
# 深さごとに、両方の木にあるノードの LP 値の差・固定数の差
print('-- by depth: n, median(HiGHS lp - ours lp), median nfix diff, leaf share ours/HiGHS')
by = C.defaultdict(list)
for e, h in m: by[e['depth'] // 5 * 5].append((e, h))
for d in sorted(by):
    v = by[d]
    dl = [h['lp'] - off - e['obj'] for e, h in v if e['obj'] == e['obj'] and h['lp'] < 1e20]
    nf = [h['nfix'] - e['nfix'] for e, h in v]
    lo = sum(e['out'] != 'br' for e, h in v) / len(v)
    lh = sum(h['res'] != 5 and h['res'] != 3 for e, h in v) / len(v)
    print(d, len(v), round(st.median(dl), 1) if dl else '-', st.median(nf), round(lo, 2), round(lh, 2))

import sys, re, collections as C
seq = sys.argv[1]
h = [l for l in open('hdet.err', errors='replace') if l.startswith('HPD %s ' % seq) or l.strip() == 'HPD %s' % seq][0].split()[2:]
o = [l for l in open('er_det.log', errors='replace') if l.startswith('ERD %s ' % seq)][0].split()[2:]
pat = re.compile(r'^(\d+)([ud])([-\d.e+]+)(\w)$')
def parse(xs):
    d = {}
    for x in xs:
        m = pat.match(x)
        if not m: continue
        j, ud, v, r = int(m.group(1)), m.group(2), float(m.group(3)), m.group(4)
        d[(j, ud)] = (v, r)
    return d
H, O = parse(h), parse(o)
print('HiGHS', len(H), C.Counter(r for v, r in H.values()))
print('ours ', len(O), C.Counter(r for v, r in O.values()))
onlyH = {k: v for k, v in H.items() if k not in O}
print('only in HiGHS', len(onlyH), C.Counter(r for v, r in onlyH.values()))
onlyO = {k: v for k, v in O.items() if k not in H}
print('only in ours', len(onlyO), C.Counter(r for v, r in onlyO.values()))
L = sorted(k for k, v in onlyH.items() if v[1] == 'L')
print('HiGHS L only cols (first 30):', ' '.join('%d%s%g' % (k[0], k[1], H[k][0]) for k in L[:30]))

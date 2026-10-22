# 同じノード (経路) での整数列の固定を、HiGHS (HPD 行) と ENOMOTO (ERD 行) で列ごとに並べる
# 使い方: python3 node_detail.py <replay.txt> <HiGHS の標準エラー> <ENOMOTO の標準エラー>
import sys
def key(path):
    return tuple(sorted((p[:p.index('u') if 'u' in p else p.index('d')], 'u' in p) for p in path))
seq2key = {}
for l in open(sys.argv[1]):
    t = l.split()
    seq2key[int(t[0])] = key([x for x in t[7:] if not x.startswith('#')])
H = {}
last = None
for l in open(sys.argv[2]):
    t = l.split()
    if not t: continue
    if t[0] == 'HP':
        last = key([x for x in t[10:] if not x.startswith('#')])
    elif t[0] == 'HPD' and last is not None:
        H[last] = t[2:]
def parse(chs):
    d = {}
    for c in chs:
        r = c[-1]; c = c[:-1]
        k = c.index('u') if 'u' in c else c.index('d')
        d[int(c[:k])] = (c[k], float(c[k+1:]), r)
    return d
for l in open(sys.argv[3]):
    t = l.split()
    if not t or t[0] != 'ERD': continue
    seq = int(t[1]); k = seq2key[seq]
    if k not in H: continue
    e = parse(t[2:]); h = parse(H[k])
    only_h = sorted((j, v) for j, v in h.items() if j not in e)
    only_e = sorted((j, v) for j, v in e.items() if j not in h)
    diffv = sorted((j, h[j], e[j]) for j in h if j in e and h[j][1] != e[j][1])
    print(f"seq {seq} depth {len(k)}: HiGHS {len(h)} ENOMOTO {len(e)} common {len(set(h) & set(e))}")
    print("   only HiGHS:", ' '.join(f"{j}{v[0]}{v[1]:g}{v[2]}" for j, v in only_h))
    print("   only ENOMOTO:", ' '.join(f"{j}{v[0]}{v[1]:g}{v[2]}" for j, v in only_e))
    if diffv: print("   different value:", diffv)

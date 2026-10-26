"""paircmp.py <元の mps> <HiGHS の .subst> <ENOMOTO の .pairs> [ENOMOTO の前処理の後の mps (固定の確認)]
HiGHS の clique の同値の代入 (消す列 残す列 val: val=1 なら同じ、0 なら補) と ENOMOTO の同値な組 (j k neg) を、
符号つきの union-find の同値類で突き合わせる。どちらにしかない組・向きの食い違いを数える。"""
import sys, highspy, collections
h = highspy.Highs(); h.setOptionValue('output_flag', False); h.readModel(sys.argv[1])
lp = h.getLp(); names = list(lp.col_names_); idx = {nm: j for j, nm in enumerate(names)}
class UF:
    def __init__(s): s.p = {}; s.par = {}
    def find(s, x):
        if x not in s.p: s.p[x] = x; s.par[x] = 0
        if s.p[x] == x: return x, 0
        r, q = s.find(s.p[x]); s.par[x] ^= q; s.p[x] = r; return r, s.par[x]
    def union(s, a, b, neg):  # x_a = x_b (neg=0) or 1-x_b (neg=1)
        ra, pa = s.find(a); rb, pb = s.find(b)
        if ra == rb: return (pa ^ pb) == neg
        s.p[ra] = rb; s.par[ra] = pa ^ pb ^ neg; return True
    def rel(s, a, b):
        if a not in s.p or b not in s.p: return None
        ra, pa = s.find(a); rb, pb = s.find(b)
        return None if ra != rb else pa ^ pb
hp = []
for l in open(sys.argv[2]):
    if l.startswith('#'): continue
    a, b, v, *_ = l.split(); hp.append((idx[a], idx[b], 1 - int(v)))
ep = []
for l in open(sys.argv[3]):
    j, k, neg = map(int, l.split()); ep.append((j, k, neg))
H, E = UF(), UF()
for a, b, n in hp: assert H.union(a, b, n)
for a, b, n in ep: assert E.union(a, b, n)
def classes(U):
    c = collections.defaultdict(list)
    for x in list(U.p): c[U.find(x)[0]].append(x)
    return [v for v in c.values() if len(v) > 1]
hc, ec = classes(H), classes(E)
print(f"HiGHS: 組 {len(hp)} (向き 同じ {sum(1 for _,_,n in hp if n==0)}・補 {sum(n for _,_,n in hp)})、類 {len(hc)}、列 {sum(len(c) for c in hc)}、消える列 {sum(len(c)-1 for c in hc)}")
print(f"ENOMOTO: 組 {len(ep)} (同じ {sum(1 for _,_,n in ep if n==0)}・補 {sum(n for _,_,n in ep)})、類 {len(ec)}、列 {sum(len(c) for c in ec)}、消える列 {sum(len(c)-1 for c in ec)}")
for nm, P, U in (("HiGHS の組で ENOMOTO の類にない", hp, E), ("ENOMOTO の組で HiGHS の類にない", ep, H)):
    same = sum(1 for a, b, n in P if U.rel(a, b) == n); opp = sum(1 for a, b, n in P if U.rel(a, b) is not None and U.rel(a, b) != n)
    print(f"{nm}: {sum(1 for a,b,n in P if U.rel(a,b) is None)} (一致 {same}、向きの食い違い {opp})")
# 片方にしかない組の列の様子
def kind(j): return names[j][:3]
miss = [(a, b, n) for a, b, n in ep if H.rel(a, b) is None]
print("ENOMOTO だけの組の例:", [(names[a], names[b], n) for a, b, n in miss[:8]])
print("ENOMOTO だけの組の列の接頭辞:", collections.Counter(kind(a) + '/' + kind(b) for a, b, n in miss).most_common(6))
miss = [(a, b, n) for a, b, n in hp if E.rel(a, b) is None]
print("HiGHS だけの組の例:", [(names[a], names[b], n) for a, b, n in miss[:8]])
# 類の大きさ
print("類の大きさ HiGHS:", collections.Counter(len(c) for c in hc).most_common(8))
print("類の大きさ ENOMOTO:", collections.Counter(len(c) for c in ec).most_common(8))
# ENOMOTO の類の列のうち HiGHS の類に入っていない列 (HiGHS では消えない)
he = set(H.p); ee = set(E.p)
print(f"列: 両方の類にある {len(he & ee)}、HiGHS だけ {len(he - ee)}、ENOMOTO だけ {len(ee - he)}")
if len(sys.argv) > 4:
    hh = highspy.Highs(); hh.setOptionValue('output_flag', False); hh.readModel(sys.argv[4]); l2 = hh.getLp()
    n2 = {nm: j for j, nm in enumerate(l2.col_names_)}
    fixed = set()
    for nm, j in n2.items():
        if l2.col_lower_[j] == l2.col_upper_[j]: fixed.add(int(nm[1:]))
    print(f"ENOMOTO の前処理の後 (書き出し) で固定の列 {len(fixed)}; HiGHS だけの類の列のうち固定 {len((he-ee) & fixed)}")

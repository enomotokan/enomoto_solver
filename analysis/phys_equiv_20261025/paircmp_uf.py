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

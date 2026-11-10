"""ENOMOTO の検出グラフ (src/mip/sympre.rs の dump、ENOMOTO_MIP_DEBUG_DUMP_SYMGRAPH) を自前の比べで調べる (2026-11-10)。
外部の検出プログラム (dejavu など) の実行が許されなかったので、その代わりの自前の参照:
(a) 色の細分化の安定な分割 (1-WL) の列の組 = 軌道の上界 (軌道は組の中に収まる)。ENOMOTO の軌道 (生成元 + 双子) が組より細かければ候補。
(b) 割れている組の中で、ENOMOTO で別の軌道にある対 (a, b) ごとに、a を b に写す自己同型を後戻りのある個別化・細分化の木で全部探す
    (節点の上限つき)。見つかれば見落とし (確かめつき)、木を探し尽くして無ければ「軌道ではない」、上限なら「不明」。
(c) ENOMOTO の生成元が問題の対称性か (列の属性と行の多重集合) を確かめ直す。
使い方: python refcheck.py dump.txt [秒の上限 (既定 120)] → JSON 1 行"""
import json, sys, time
import numpy as np

M1, M2 = np.uint64(0xBF58476D1CE4E5B9), np.uint64(0x94D049BB133111EB)

def mixv(x):
    x = x.astype(np.uint64, copy=True)
    with np.errstate(over="ignore"):
        x ^= x >> np.uint64(30); x *= M1; x ^= x >> np.uint64(27); x *= M2; x ^= x >> np.uint64(31)
    return x

def load(path):
    f = open(path)
    n, m, secs = f.readline().split(); n, m = int(n), int(m)
    cols, rlo, rup, rows, twins, gens = [], [], [], [], [], []
    for line in f:
        t = line.split()
        if not t: continue
        if t[0] == "C": cols.append((t[1], t[2], t[3], t[4]))
        elif t[0] == "R":
            rlo.append(t[1]); rup.append(t[2]); k = int(t[3])
            rows.append([(int(t[4 + 2 * q]), t[5 + 2 * q]) for q in range(k)])
        elif t[0] == "T": twins.append([int(x) for x in t[2:]])
        elif t[0] == "G":
            g = list(range(n)); v = [int(x) for x in t[1:]]
            for q in range(0, len(v), 2): g[v[q]] = v[q + 1]
            gens.append(g)
    return n, m, float(secs), cols, rlo, rup, rows, twins, gens

class UF:
    def __init__(s, n): s.p = list(range(n))
    def f(s, x):
        while s.p[x] != x: s.p[x] = s.p[s.p[x]]; x = s.p[x]
        return x
    def u(s, a, b):
        a, b = s.f(a), s.f(b)
        if a != b: s.p[a] = b

def main():
    path = sys.argv[1]; tlim = float(sys.argv[2]) if len(sys.argv) > 2 else 120.0
    t0 = time.time()
    n, m, secs, cols, rlo, rup, rows, twins, gens = load(path)
    ids = {}
    def cid(x): return ids.setdefault(x, len(ids) + 1)
    E_r = np.array([i for i in range(m) for _ in rows[i]], dtype=np.int64)
    E_c = np.array([j for r in rows for j, _ in r], dtype=np.int64)
    E_a = np.array([cid(("a", a)) for r in rows for _, a in r], dtype=np.uint64)
    deg = np.bincount(E_c, minlength=n) if len(E_c) else np.zeros(n, dtype=np.int64)
    c0 = np.array([cid(("c",) + c) for c in cols], dtype=np.uint64)
    for j in range(n):
        if cols[j][1] == cols[j][2] and deg[j] == 0: c0[j] = np.uint64(cid(("fixed_empty", j)))
    r0 = np.array([cid(("r", rlo[i], rup[i])) for i in range(m)], dtype=np.uint64)
    # 辺を行の順・列の順に並べた添字 (reduceat 用)
    o_r = np.argsort(E_r, kind="stable"); o_c = np.argsort(E_c, kind="stable")
    rs = np.searchsorted(E_r[o_r], np.arange(m)); cs = np.searchsorted(E_c[o_c], np.arange(n))
    def seg_sum(vals, order, starts, cnt_len, total):
        out = np.zeros(cnt_len, dtype=np.uint64)
        if len(vals) == 0: return out
        v = vals[order]
        nonempty = np.zeros(cnt_len, dtype=bool)
        ends = np.append(starts[1:], len(v)); nonempty = ends > starts
        idx = starts[nonempty]
        with np.errstate(over="ignore"):
            out[nonempty] = np.add.reduceat(v, idx)
        return out
    def refine(c, r):
        prev = -1
        while True:
            with np.errstate(over="ignore"):
                r = mixv(r * np.uint64(31) + seg_sum(mixv(c[E_c] * np.uint64(1000003) + E_a), o_r, rs, m, None) + np.uint64(7))
                c = mixv(c * np.uint64(37) + seg_sum(mixv(r[E_r] * np.uint64(999983) + E_a), o_c, cs, n, None) + np.uint64(11))
            k = len(np.unique(c)) + len(np.unique(r))
            if k == prev: return c, r
            prev = k
    c, r = refine(c0, r0)
    # 確かめ (列の属性と行の多重集合)
    base = {}
    for i in range(m):
        key = (rlo[i], rup[i], tuple(sorted(rows[i]))); base[key] = base.get(key, 0) + 1
    def verify(g):
        if any(cols[j] != cols[g[j]] for j in range(n)): return False
        cnt = dict(base)
        for i in range(m):
            key = (rlo[i], rup[i], tuple(sorted((g[j], a) for j, a in rows[i])))
            if cnt.get(key, 0) == 0: return False
            cnt[key] -= 1
        return True
    bad = sum(1 for g in gens if not verify(g))
    ue = UF(n)
    for g in gens:
        for j in range(n):
            if g[j] != j: ue.u(j, g[j])
    for t in twins:
        for j in t[1:]: ue.u(t[0], j)
    cells = {}
    for j in range(n): cells.setdefault(int(c[j]), []).append(j)
    cells = [v for v in cells.values() if len(v) >= 2]
    wl_cols = sum(len(v) for v in cells)
    eroot = [ue.f(j) for j in range(n)]
    from collections import Counter
    esz = Counter(eroot)
    e_cols = sum(1 for j in range(n) if esz[eroot[j]] >= 2)
    split = [v for v in cells if len({eroot[j] for j in v}) > 1]
    # (b) 割れた組の対を後戻りで探す
    found, nonorb, unknown, nodes_total = 0, 0, 0, 0
    extra_gens = []
    deadline = t0 + tlim
    def groups(cc):
        d = {}
        for j, x in enumerate(cc.tolist()): d.setdefault(x, []).append(j)
        return d
    def search(ca, ra, cb, rb, budget):
        # 戻り値: (置換 or None, 探し尽くしたか)
        budget[0] -= 1
        if budget[0] < 0 or time.time() > deadline: return None, False
        ga, gb = groups(ca), groups(cb)
        if len(ga) != len(gb) or any(len(v) != len(gb.get(k, ())) for k, v in ga.items()):
            return None, True
        if len(ga) == n:
            g = [0] * n
            for k, v in ga.items(): g[v[0]] = gb[k][0]
            return (g, True) if verify(g) else (None, True)
        # 目標の組: 最小の大きさ (2 以上) のうち、先頭の列が最小のもの
        cand = [v for v in ga.values() if len(v) >= 2]
        tgt = min(cand, key=lambda v: (len(v), v[0])); key = int(ca[tgt[0]])
        x = tgt[0]; complete = True
        ys = gb[key]
        for y in ([x] if x in ys else []) + [y for y in ys if y != x]:
            ca2 = ca.copy(); cb2 = cb.copy()
            ca2[x] = mixv(np.array([ca2[x] ^ np.uint64(0xA5A5)]))[0]; cb2[y] = mixv(np.array([cb2[y] ^ np.uint64(0xA5A5)]))[0]
            a2, ra2 = refine(ca2, ra); b2, rb2 = refine(cb2, rb)
            g, comp = search(a2, ra2, b2, rb2, budget)
            if g is not None: return g, True
            complete &= comp
        return None, complete
    for v in split:
        a = v[0]
        for b in v[1:]:
            if time.time() > deadline: break
            if ue.f(a) == ue.f(b): continue
            ca = c.copy(); cb = c.copy()
            ca[a] = mixv(np.array([ca[a] ^ np.uint64(0xA5A5)]))[0]; cb[b] = mixv(np.array([cb[b] ^ np.uint64(0xA5A5)]))[0]
            a2, ra2 = refine(ca, r); b2, rb2 = refine(cb, r)
            budget = [200]
            g, comp = search(a2, ra2, b2, rb2, budget)
            nodes_total += 200 - budget[0]
            if g is not None:
                found += 1; extra_gens.append(g)
                for j in range(n):
                    if g[j] != j: ue.u(j, g[j])
            elif comp: nonorb += 1
            else: unknown += 1
    rroot = [ue.f(j) for j in range(n)]; rsz = Counter(rroot)
    ref_cols = sum(1 for j in range(n) if rsz[rroot[j]] >= 2)
    ref_orbits = sum(1 for k, s in rsz.items() if s >= 2)
    e_orbits = sum(1 for k, s in esz.items() if s >= 2)
    # 軌道の大きさの積 (log10) を群の位数の目安として
    lg = lambda sz: float(sum(np.log10(s) for s in sz.values() if s >= 2))
    print(json.dumps(dict(n=n, m=m, nnz=int(len(E_c)), enomoto_secs=secs, gens=len(gens), twins=len(twins), bad_gens=bad,
                          wl_cells=len(cells), wl_cols=wl_cols, e_orbits=e_orbits, e_cols=e_cols, split_cells=len(split),
                          missed_found=found, nonorbit_pairs=nonorb, unknown_pairs=unknown, ref_orbits=ref_orbits, ref_cols=ref_cols,
                          log10_orbprod_e=round(lg(esz), 3), log10_orbprod_ref=round(lg(rsz), 3),
                          timed_out=time.time() > deadline, secs=round(time.time() - t0, 2))))

main()

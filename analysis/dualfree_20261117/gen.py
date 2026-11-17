"""sat_rens の形 (時刻の連続列 + 差の不等式 + 大きい M の行 + 0-1 列) の乱数問題を作る。

問題は (lo, up, c, it, rows, rlo, rup) で、rows[i] は [(列, 係数)]。最小化。
時刻の列の境界は、行から導ける値ちょうど (非厳密の場合) を基本に、許容の境目 (±1e-10, ±1e-8, ±1e-6) や
導けない厳しい値、無限も混ぜる。"""
import random
inf = float('inf')

def implied(j, rows, rlo, rup, lo, up):
    """列 j の、各行 1 本から導ける (下限, 上限) の最も厳しい値 (他の列は箱の範囲で動く)"""
    L, U = -inf, inf
    for r, l, u in zip(rows, rlo, rup):
        a = dict(r).get(j)
        if a is None:
            continue
        mn = mx = 0.0
        for k, v in r:
            if k == j:
                continue
            p, q = (v * lo[k], v * up[k]) if v > 0 else (v * up[k], v * lo[k])
            mn += p; mx += q
        if u < inf and mn > -inf:   # a x_j <= u - mn
            b = (u - mn) / a
            if a > 0: U = min(U, b)
            else: L = max(L, b)
        if l > -inf and mx < inf:   # a x_j >= l - mx
            b = (l - mx) / a
            if a > 0: L = max(L, b)
            else: U = min(U, b)
    return L, U

def gen(seed, lp_only=False):
    g = random.Random(seed)
    K = g.randint(2, 8)            # 時刻の列
    B = g.randint(1, 8)            # 0-1 列
    I = g.randint(0, 2)            # 一般の整数列
    S = g.randint(0, 3)            # 他の連続列
    n = K + B + I + S
    T = list(range(K)); Z = list(range(K, K + B)); G = list(range(K + B, K + B + I)); C = list(range(K + B + I, n))
    lo = [0.0] * n; up = [0.0] * n; it = [False] * n; c = [0.0] * n
    scale = g.choice([1.0, 10.0, 1e3, 1e5])
    for j in T:
        lo[j], up[j] = -scale * g.random(), scale * (1 + g.random())
        c[j] = g.choice([0.0, 0.0, 0.0, 0.0, 1.0, -1.0, round(g.uniform(-3, 3), 2)])
    for j in Z:
        lo[j], up[j] = 0.0, 1.0; it[j] = not lp_only; c[j] = round(g.uniform(-10, 10))
    for j in G:
        lo[j], up[j] = 0.0, float(g.randint(1, 4)); it[j] = not lp_only; c[j] = round(g.uniform(-5, 5))
    for j in C:
        lo[j] = round(g.uniform(0, scale), 3); up[j] = lo[j] + round(g.uniform(0, scale / 10), 3); c[j] = round(g.uniform(-2, 2), 2)
    # 実行可能な点
    x0 = [g.uniform(lo[j], up[j]) for j in range(n)]
    for j in Z + G:
        x0[j] = float(round(x0[j]))
    rows = []; rlo = []; rup = []
    def add(r, kind):
        r = [(j, v) for j, v in r if v != 0.0]
        d = {}
        for j, v in r:
            d[j] = d.get(j, 0.0) + v
        r = sorted((j, v) for j, v in d.items() if v != 0.0)
        if not r:
            return
        act = sum(v * x0[j] for j, v in r)
        if kind == 'le':
            rlo.append(-inf); rup.append(act + g.choice([0.0, g.uniform(0, scale / 20)]))
        elif kind == 'ge':
            rlo.append(act - g.choice([0.0, g.uniform(0, scale / 20)])); rup.append(inf)
        elif kind == 'eq':
            rlo.append(act); rup.append(act)
        else:
            w = g.uniform(0, scale / 20); rlo.append(act - w); rup.append(act + w)
        rows.append(r)
    coef = lambda: g.choice([1.0, 1.0, 1.0, 0.426, 2.0, 0.5])
    # 差の不等式 (0-1 列が混ざることも)
    for _ in range(g.randint(K, 3 * K)):
        a, b = g.sample(T, 2) if K >= 2 else (T[0], T[0])
        r = [(a, coef()), (b, -coef())]
        if g.random() < 0.4 and Z:
            r.append((g.choice(Z), g.choice([-1, 1]) * g.uniform(0, scale / 5)))
        if g.random() < 0.2 and C:
            r.append((g.choice(C), g.choice([-1.0, 1.0])))
        add(r, g.choice(['le', 'le', 'le', 'ge', 'rng']))
    # 大きい M の行: t − (他の時刻か連続) + M z ≤ M + d
    for _ in range(g.randint(1, K + 1)):
        t = g.choice(T)
        M = g.choice([1e3, 1e4, 2e5]) * (scale / 1e3 if scale > 1e3 else 1.0)
        r = [(t, g.choice([1.0, -1.0]))]
        if C or len(T) > 1:
            r.append((g.choice(C + [k for k in T if k != t]), g.choice([1.0, -1.0])))
        if Z:
            r.append((g.choice(Z), M))
        add(r, g.choice(['le', 'le', 'ge']))
    # 0-1 列・整数列の行 (選択・容量)
    for _ in range(g.randint(1, 4)):
        k = g.sample(Z + G, min(len(Z + G), g.randint(1, 4)))
        r = [(j, float(g.randint(1, 5))) for j in k]
        if g.random() < 0.3:
            r.append((g.choice(T), g.uniform(-1, 1)))
        add(r, g.choice(['le', 'ge', 'eq']))
    # 時刻の列が等式に現れる場合 (対象外になるはず)
    if g.random() < 0.15:
        add([(g.choice(T), 1.0), (g.choice(Z + C + T), -1.0)], 'eq')
    # 時刻の列の境界: 行から導ける値ちょうどを基本に、境目と厳しい値と無限を混ぜる
    for j in T:
        L, U = implied(j, rows, rlo, rup, lo, up)
        for side in (0, 1):
            v = L if side == 0 else U
            mode = g.choice(['exact'] * 5 + ['eps10', 'eps8', 'eps6', 'tight', 'inf', 'keep'])
            if not (v > -inf and v < inf):
                continue
            if mode == 'exact': w = v
            elif mode == 'eps10': w = v + g.choice([-1, 1]) * 1e-10 * max(1, abs(v))
            elif mode == 'eps8': w = v + g.choice([-1, 1]) * 1e-8 * max(1, abs(v))
            elif mode == 'eps6': w = v + g.choice([-1, 1]) * 1e-6 * max(1, abs(v))
            elif mode == 'tight':
                w = (v + x0[j]) / 2
            elif mode == 'inf': w = -inf if side == 0 else inf
            else: continue
            if side == 0: lo[j] = min(w, x0[j])
            else: up[j] = max(w, x0[j])
    return lo, up, c, it, rows, rlo, rup

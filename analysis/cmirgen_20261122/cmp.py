"""同じ置き換え後の項で、当方の CMIR (src/mip/cuts.rs の cmir_terms) と SCIP 10.0 の SCIPcutGenerationHeuristicCMIR (cuts.c) の
δ の選び方・補数の反転・f0 の範囲を比べる: python cmp.py dump/問題.txt [...]。
dump は ENOMOTO_MIP_CMIRDUMP=N の `CMIRDUMP {"beta":..,"t":[[a, yv, yu, int], ...]}` (置き換えた後の項 a y、0 <= y <= yu、右辺 beta)。
置き換え (境界・変数上下限の選び方) は両者とも「近い方」なので共通とし、その後の部分だけを比べる。
効き目は置き換えた空間の違反量 / ノルム (両者の測り方は同じ式。SCIP の computeMIREfficacy と当方の mir_efficacy)。
規則 (それぞれの違いを 1 つずつ入れ替えて、どれが差を生むかを見る):
- ours: δ = 内側にある整数の項の |a| (項の順に最大 8 個、なければ整数の |a| の最大)、それぞれの /2,/4,/8 も。f0 ∈ [0.005, 0.995]、
  |b| <= 1e6。補数の反転は項の順に (整数・上限有限・yv > 1e-6) 最大 50 個
- scip: δ = 内側にある整数の項の a すべて (境界からの距離の大きい順)、整数にする倍率からの δ と δ2、最大の |a| + 1、1。最良の δ だけ /2,/4,/8。
  f0 ∈ [0.05, 0.999]、scale / (1 - f0) <= 1e6。補数の反転は内側の整数の項だけ、境界からの距離の大きい順、全部
"""
import json, math, sys
from fractions import Fraction
from collections import defaultdict

EPS = 1e-9


def eff(terms, beta, delta, fmin, fmax, scip_scale):
    if delta <= 0:
        return None
    b = beta / delta
    f0 = b - math.floor(b)
    if f0 < fmin or f0 > fmax:
        return None
    if scip_scale:
        if (1 / delta) / (1 - f0) > 1e6:
            return None
    elif abs(b) > 1e6:
        return None
    act, nrm = 0.0, 0.0
    for a, yv, yu, it in terms:
        at = a / delta
        if it:
            fl = math.floor(at)
            fj = at - fl
            c = fl + ((fj - f0) / (1 - f0) if fj > f0 + (EPS if scip_scale else 0.0) else 0.0)
        else:
            c = at / (1 - f0)
        act += c * yv
        nrm += c * c
    v = act - math.floor(b)
    if nrm <= 0 or v <= 1e-6:
        return None
    return v / math.sqrt(nrm)


def inside(t):
    a, yv, yu, it = t
    return it and yv > 1e-6 and yv < yu - 1e-6


def int_scalar(vals, maxden=10000, maxscale=10000.0):
    """SCIPcalcIntegralScalar の近似: 全部を整数にする最小の倍率 (分母 maxden まで、倍率 maxscale まで)"""
    den = 1
    for v in vals:
        fr = Fraction(abs(v)).limit_denominator(maxden)
        if abs(float(fr) - abs(v)) > 1e-9 * max(1.0, abs(v)):
            return None
        den = den * fr.denominator // math.gcd(den, fr.denominator)
        if den > maxscale:
            return None
    nums = [round(abs(v) * den) for v in vals]
    g = 0
    for x in nums:
        g = math.gcd(g, x)
    if g == 0:
        return None
    return den / g


def flips(terms, beta, delta, best, order, fmin, fmax, sc, cap=None):
    terms = [list(t) for t in terms]
    n = 0
    for idx in order:
        a, yv, yu, it = terms[idx]
        if cap is not None:
            n += 1
            if n > cap:
                break
        t2 = [-a, yu - yv, yu, it]
        beta2 = beta - a * yu
        old = terms[idx]
        terms[idx] = t2
        e = eff(terms, beta2, delta, fmin, fmax, sc)
        if e is not None and e > best + 1e-9:
            best, beta = e, beta2
        else:
            terms[idx] = old
    return best


def rule(terms, beta, deltas_mode="ours", fmin=0.005, fmax=0.995, sc=False, div="all", flip="ours"):
    ints = [t for t in terms if t[3]]
    if not ints:
        return None, None
    ins = [i for i, t in enumerate(terms) if inside(t)]
    if deltas_mode == "ours":
        ds = []
        for i in ins:
            d = abs(terms[i][0])
            if d > 1e-6 and not any(abs(e - d) <= 1e-9 * d for e in ds):
                ds.append(d)
            if len(ds) >= 8:
                break
        if not ds:
            m = max(abs(t[0]) for t in ints)
            if m > 0:
                ds.append(m)
    else:
        if not ins:
            return None, None
        srt = sorted(ins, key=lambda i: -min(terms[i][1], terms[i][2] - terms[i][1]))
        ds = [terms[i][0] for i in srt]
        s = int_scalar(ds)
        if s is not None:
            r = beta * s
            f = r - math.floor(r)
            if abs(f - round(f)) > 1e-6:
                if f < fmin or f > fmax:
                    s *= math.ceil(max(fmin, 1 - fmax) / min(f, 1 - f))
                    r = beta * s
                    f = r - math.floor(r)
                if fmin <= f <= fmax:
                    d = 1 / s
                    if abs(d - 1) > EPS:
                        ds.append(d)
                    if f < fmax:
                        d2 = 1 / (s * math.floor(fmax / f))
                        if abs(d - d2) > EPS and abs(d2 - 1) > EPS:
                            ds.append(d2)
        k = len(srt)
        mx = max(abs(x) for x in ds[:k])
        ds = [abs(x) for x in ds]
        ds.append(mx + 1.0)
        ds.append(1.0)
    seen = []
    best, bd = None, None
    for d in ds:
        if any(abs(d - e) <= EPS * max(1, d) for e in seen):
            continue
        seen.append(d)
        e = eff(terms, beta, d, fmin, fmax, sc)
        if e is not None and (best is None or e > best):
            best, bd = e, d
    if div == "all":
        for d in list(seen):
            for q in (2, 4, 8):
                e = eff(terms, beta, d / q, fmin, fmax, sc)
                if e is not None and (best is None or e > best):
                    best, bd = e, d / q
    else:
        if bd is not None:
            b0 = bd
            for q in (2, 4, 8):
                e = eff(terms, beta, b0 / q, fmin, fmax, sc)
                if e is not None and e > best:
                    best, bd = e, b0 / q
    if best is None:
        return None, None
    if flip == "ours":
        order = [i for i, t in enumerate(terms) if t[3] and t[2] < 1e299 and t[1] > 1e-6]
        best = flips(terms, beta, bd, best, order, fmin, fmax, sc, cap=50)
    else:
        order = sorted([i for i in ins if terms[i][2] < 1e299], key=lambda i: -min(terms[i][1], terms[i][2] - terms[i][1]))
        best = flips(terms, beta, bd, best, order, fmin, fmax, sc)
    return best, bd


RULES = {
    "ours": dict(),
    "scip": dict(deltas_mode="scip", fmin=0.05, fmax=0.999, sc=True, div="best", flip="scip"),
    # 当方から 1 つずつ SCIP 寄りにする
    "o+sdelta": dict(deltas_mode="scip"),
    "o+frac": dict(fmin=0.05, fmax=0.999, sc=True),
    "o+sflip": dict(flip="scip"),
    "o+sdelta+div": dict(deltas_mode="scip", div="best"),
    # f0 の範囲と大きさの判定を分ける
    "o+fmin": dict(fmin=0.05, fmax=0.999),
    "o+fmax": dict(fmax=0.999),
    "o+fmin_only": dict(fmin=0.05),
    "s-fmin": dict(deltas_mode="scip", fmin=0.005, fmax=0.999, sc=True, div="best", flip="scip"),
    "o+scale": dict(sc=True),
}

if __name__ == "__main__":
    for f in sys.argv[1:]:
        st = defaultdict(lambda: [0, 0.0])  # 規則 → [カットが出た数, 効き目の和]
        win = defaultdict(lambda: [0, 0])  # 規則 → [ours より 1% 超良い, 1% 超悪い]
        nrow = 0
        for line in open(f):
            d = json.loads(line.split(" ", 1)[1])
            terms = [tuple(t) for t in d["t"]]
            nrow += 1
            res = {k: rule(terms, d["beta"], **v)[0] for k, v in RULES.items()}
            for k, e in res.items():
                if e is not None:
                    st[k][0] += 1
                    st[k][1] += e
                o = res["ours"] or 0.0
                ee = e or 0.0
                if ee > o * 1.01 + 1e-9:
                    win[k][0] += 1
                elif ee < o * 0.99 - 1e-9:
                    win[k][1] += 1
        print(f"## {f} ({nrow} 行)")
        for k in RULES:
            print(f"  {k:14s} cuts {st[k][0]:6d} sum_eff {st[k][1]:10.3f} better {win[k][0]:5d} worse {win[k][1]:5d}")

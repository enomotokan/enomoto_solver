"""ENOMOTO と HiGHS の前処理後の問題の列・行の違い: coldiff.py <eno.mps> <highs_pre.mps>

- 列: 片方だけに残る列の数 (0-1 / 整数 / 連続)。ENOMOTO にだけ残る 0-1 列について、ENOMOTO の行での出方
  (行数、出る行の形) を数える。
- 行: 形 (長さ・係数・向き) ごとの本数を両方で並べる。
"""
import sys
from collections import Counter

sys.path.insert(0, __import__("os").path.dirname(__file__))
from twovar_census import load, clique_form  # noqa: E402
from twovar_vs_highs import names  # noqa: E402


def row_kind(r, l, u, binary, integ_cols):
    k = len(r)
    allb = all(binary[j] for j, _ in r)
    alli = all(j in integ_cols for j, _ in r)
    pm1 = all(abs(abs(a) - 1) < 1e-12 for _, a in r)
    sense = "E" if l == u else ("R" if l > -1e30 and u < 1e30 else ("L" if u < 1e30 else "G"))
    cf = clique_form(r, l, u, binary) if allb and pm1 else []
    if cf:
        t = "clique"
    elif allb and pm1:
        t = "bin±1"
    elif allb:
        t = "bin"
    elif alli:
        t = "int"
    else:
        t = "mixed"
    lb = "2" if k == 2 else "3" if k == 3 else "4-10" if k <= 10 else "11-100" if k <= 100 else ">100"
    return (t, sense, lb)


def summarize(path, rename):
    n, m, rows, rl, ru, binary, lo, up, integ = load(path)
    import highspy
    ints = {j for j in range(n) if integ[j] == highspy.HighsVarType.kInteger}
    used = set(j for r in rows for j, _ in r)
    kinds = Counter(row_kind(r, rl[i], ru[i], binary, ints) for i, r in enumerate(rows) if r)
    cols = {rename[j]: ("B" if binary[j] else "I" if j in ints else "C") for j in used if lo[j] < up[j]}
    return kinds, cols, (n, m, rows, rl, ru, binary, lo, up, integ)


def main():
    eno, hi = sys.argv[1], sys.argv[2]
    en = names(eno)
    eren = [f"c{int(x[1:]) + 1}" for x in en]
    hn = names(hi)
    ek, ec, edata = summarize(eno, eren)
    hk, hc, _ = summarize(hi, hn)
    print("columns ENOMOTO", Counter(ec.values()), "HiGHS", Counter(hc.values()))
    only_e = {c: t for c, t in ec.items() if c not in hc}
    only_h = {c: t for c, t in hc.items() if c not in ec}
    print("only in ENOMOTO", Counter(only_e.values()), " only in HiGHS", Counter(only_h.values()))
    both_diff = Counter((ec[c], hc[c]) for c in ec if c in hc and ec[c] != hc[c])
    print("type changed", dict(both_diff))
    print(f"{'kind':32s} {'ENO':>7s} {'HiGHS':>7s}")
    for k in sorted(set(ek) | set(hk)):
        print(f"{str(k):32s} {ek.get(k, 0):7d} {hk.get(k, 0):7d}")
    # ENOMOTO にだけ残る 0-1 列の出方
    n, m, rows, rl, ru, binary, lo, up, integ = edata
    import highspy
    ints = {j for j in range(n) if integ[j] == highspy.HighsVarType.kInteger}
    colrows = [[] for _ in range(n)]
    for i, r in enumerate(rows):
        for j, a in r:
            colrows[j].append(i)
    oe = [j for j in range(n) if eren[j] in only_e and binary[j]]
    deg = Counter(min(len(colrows[j]), 10) for j in oe)
    print("ENOMOTO-only binaries: #rows per column", sorted(deg.items()))
    rk = Counter()
    for j in oe:
        for i in colrows[j]:
            rk[row_kind(rows[i], rl[i], ru[i], binary, ints)] += 1
    print("ENOMOTO-only binaries appear in rows of kind:", rk.most_common(12))
    cost = Counter("cost" if 0 else "x" for _ in [])


if __name__ == "__main__":
    main()

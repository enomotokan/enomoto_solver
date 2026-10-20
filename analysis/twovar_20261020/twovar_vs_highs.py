"""ENOMOTO の前処理後に残る 2 変数の ±1 の行が HiGHS の前処理後でどうなっているかを、列の名前で突き合わせる:
twovar_vs_highs.py <eno.mps (列 C<j>、0 始まり)> <highs_pre.mps (元の名前 c<j+1>)> [orig.mps]

ENOMOTO の 2 変数の行 (文字の和 <= 1 の形) ごとに、HiGHS の前処理後で
  both  : 2 列とも残っている
    same_row : 2 文字を両方含むクリーク行 (文字の和 <= 1) がある (HiGHS は長い行にまとめた)
    row2     : 同じ 2 変数の行がそのままある
    none     : どちらもない (HiGHS は別の形で表している / 消した)
  one   : 1 列だけ残っている (もう 1 列は消えた: 固定・代入・支配)
  zero  : 2 列とも消えた
を数える。
"""
import sys
from collections import Counter, defaultdict

sys.path.insert(0, __import__("os").path.dirname(__file__))
from twovar_census import load, clique_form  # noqa: E402

import highspy


def names(path):
    h = highspy.Highs()
    h.setOptionValue("output_flag", False)
    h.readModel(path)
    return list(h.getLp().col_names_)


def two_var_rows(path, rename):
    n, m, rows, rl, ru, binary, lo, up, integ = load(path)
    out = []
    cl = []
    for i, r in enumerate(rows):
        for ls in clique_form(r, rl[i], ru[i], binary):
            lits = tuple(sorted((rename[l // 2], l % 2) for l in ls))
            (out if len(r) == 2 else cl).append(lits)
    present = {rename[j] for j in range(n) if lo[j] < up[j] and any(True for _ in [0])}
    return out, cl, present, n


def main():
    eno, hi = sys.argv[1], sys.argv[2]
    en = names(eno)
    hn = names(hi)
    # ENOMOTO の C<j> -> 元の c<j+1>
    eren = [f"c{int(x[1:]) + 1}" for x in en]
    e2, ecl, _, _ = two_var_rows(eno, eren)
    h2, hcl, _, _ = two_var_rows(hi, hn)
    # ENOMOTO で使われている列 (前処理後に行に出る列)
    hset = set(hn)
    h2set = set(h2)
    by_lit = defaultdict(list)
    for c, ls in enumerate(hcl):
        for l in ls:
            by_lit[l].append(c)
    hcl_sets = [set(ls) for ls in hcl]
    st = Counter()
    sizes = Counter()
    for e in e2:
        (a, sa), (b, sb) = e
        k = {0: "pack", 1: "impl", 2: "cover"}[sa + sb]
        pa, pb = a in hset, b in hset
        if pa and pb:
            if e in h2set:
                st[(k, "both:row2")] += 1
            else:
                cs = [c for c in by_lit.get((a, sa), ()) if (b, sb) in hcl_sets[c]]
                if cs:
                    st[(k, "both:same_row")] += 1
                    sizes[max(len(hcl[c]) for c in cs)] += 1
                else:
                    st[(k, "both:none")] += 1
        elif pa or pb:
            st[(k, "one")] += 1
        else:
            st[(k, "zero")] += 1
    print(f"ENOMOTO 2-var rows {len(e2)}, HiGHS 2-var rows {len(h2)}, HiGHS long clique rows {len(hcl)}, HiGHS cols {len(hn)}")
    for kk, v in sorted(st.items()):
        print(f"  {kk[0]:5s} {kk[1]:14s} {v}")
    print("  sizes of HiGHS rows absorbing:", sorted(sizes.items())[:20])
    # HiGHS の 2 変数の行は ENOMOTO にあるか
    eset = set(e2)
    print(f"HiGHS 2-var rows also in ENOMOTO: {sum(1 for e in h2 if e in eset)} / {len(h2)}")


if __name__ == "__main__":
    main()

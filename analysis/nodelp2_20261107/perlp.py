"""ENOMOTO の `HO4DIAG: node LP` の行 (logs/<問題>_<設定>_<種>.log、ENOMOTO_MIP_HO4_DIAG) から、木のノードの LP 1 回ごとの反復の分布を
種類 (最初の LP の kind: 0 潜りの子・兄弟、1 待ち行列、2 共有の待ち行列。resolve>0 は解き直し) と状態ごとに出す。
使い方: python perlp.py ログ..."""
import re, sys, statistics as S
pat = re.compile(r"HO4DIAG: node LP tid (\d+) node (\d+) depth (\d+) resolve (\d+) restored (\w+) kind (\d+) rows (\d+) .* first (\w+) (\d+) iters ([\d.]+)s")
for fn in sys.argv[1:]:
    g = {}
    for l in open(fn, errors="replace"):
        m = pat.search(l)
        if not m:
            continue
        res = int(m.group(4)); kind = "resolve" if res else ("restored" if m.group(5) == "true" else "cont") + f"/k{m.group(6)}"
        g.setdefault(kind, []).append((int(m.group(9)), m.group(8), int(m.group(3)), float(m.group(10))))
        g.setdefault("all-first" if not res else "all-resolve", []).append((int(m.group(9)), m.group(8), int(m.group(3)), float(m.group(10))))
    print(fn)
    for k, v in sorted(g.items()):
        it = sorted(a[0] for a in v); n = len(it)
        st = {}
        for a in v:
            st.setdefault(a[1], [0, 0]); st[a[1]][0] += 1; st[a[1]][1] += a[0]
        print(f"  {k}: n {n} it/LP {S.mean(it):.1f} median {it[n//2]} p90 {it[int(.9*n)]} max {it[-1]} depth {S.mean(a[2] for a in v):.1f} "
              f"zero {sum(1 for x in it if x == 0)} status " + ", ".join(f"{s}: {c[0]} ({c[1]/c[0]:.0f} it)" for s, c in sorted(st.items())))

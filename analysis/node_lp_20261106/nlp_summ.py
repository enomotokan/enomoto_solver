"""計装した HiGHS の `NLP` の行 (logs/<問題>_highs.nlp.gz) の要約: 主の MIP (sub 0) のノードの LP の反復・行・時間を、状態ごと・反復の分位で。
引数の前に all (全部)・tree (ヒューリスティクスの外の最初の LP)・heur (ヒューリスティクスの潜り)・rec (境界の固定の後の解き直し) を置くと絞る。"""
import gzip, sys, statistics as S
sel = "all"
for p in sys.argv[1:]:
    if p in ("all", "tree", "heur", "rec"):
        sel = p; continue
    rows = []
    for l in gzip.open(f"logs/{p}_highs.nlp.gz", "rt"):
        t = l.split(); d = dict(zip(t[1::2], t[2::2]))
        if d.get("sub") != "0":
            continue
        if sel == "tree" and (d.get("heur", "0") != "0" or d.get("rec", "0") != "0"):
            continue
        if sel == "heur" and d.get("heur", "0") == "0":
            continue
        if sel == "rec" and (d.get("heur", "0") != "0" or d.get("rec", "0") == "0"):
            continue
        rows.append((int(d["it"]), int(d["rows"]), int(d["mrows"]), float(d["secs"]), int(d["st"]), int(d["d"])))
    if not rows:
        print(p, "no NLP"); continue
    n = len(rows); it = [r[0] for r in rows]
    st = {}
    for r in rows:
        st.setdefault(r[4], [0, 0]); st[r[4]][0] += 1; st[r[4]][1] += r[0]
    q = sorted(it)
    print(f"{p} [{sel}]: LPs {n} it/LP {sum(it)/n:.1f} (median {q[n//2]}, p90 {q[int(n*.9)]}, max {q[-1]}) rows {sum(r[1] for r in rows)/n:.1f} cutrows {sum(r[1]-r[2] for r in rows)/n:.1f} "
          f"ms/LP {1e3*sum(r[3] for r in rows)/n:.2f} us/it {1e6*sum(r[3] for r in rows)/max(1,sum(it)):.0f} zero-iter {sum(1 for x in it if x==0)} status {{st: [n, it/LP]}} {{{', '.join(f'{k}: [{v[0]}, {v[1]/v[0]:.1f}]' for k, v in sorted(st.items()))}}}")

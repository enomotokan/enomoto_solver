"""計装した HiGHS の NLP の行を、木の最初の LP (ヒューリスティクス・解き直しを除く) について、基底を戻した直後 (restored 1) とそれ以外 (親の続きの子) に分けて数える。
使い方: python highs_restored.py 問題..."""
import gzip, sys, statistics as S
for p in sys.argv[1:]:
    g = {}
    for l in gzip.open(f"logs/{p}_highs.nlp.gz", "rt"):
        t = l.split(); x = dict(zip(t[1::2], t[2::2]))
        if x["sub"] != "0" or x.get("heur", "0") != "0" or x.get("rec", "0") != "0":
            continue
        g.setdefault(x.get("restored", "?"), []).append((int(x["it"]), float(x["secs"]), int(x["rows"])))
    for k, v in sorted(g.items()):
        it = [a[0] for a in v]
        print(f"{p} restored {k}: n {len(v)} it/LP {S.mean(it):.1f} median {S.median(it)} p90 {sorted(it)[int(.9*len(it))]} ms/LP {1e3*S.mean(a[1] for a in v):.2f} rows {S.mean(a[2] for a in v):.0f}")

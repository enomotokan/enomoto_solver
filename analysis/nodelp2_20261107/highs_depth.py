"""計装した HiGHS の NLP の行 (gz) から、木の最初の LP の深さの帯ごとの回数/平均反復と、基底を戻したか (restored) ごとの数を出す。
使い方: python highs_depth.py nlp.gz..."""
import gzip, sys, collections
for fn in sys.argv[1:]:
    rows = []
    for l in gzip.open(fn, "rt"):
        t = l.split(); d = dict(zip(t[1::2], t[2::2]))
        if d["sub"] != "0" or d.get("heur", "0") != "0" or d.get("rec", "0") != "0":
            continue
        rows.append((int(d["d"]), int(d["it"]), d["st"], d.get("restored", d.get("rst", "?"))))
    b = collections.defaultdict(list)
    for dd, it, st, rs in rows:
        b[min(dd // 10 * 10, 60)].append(it)
    print(fn, len(rows), " ".join(f"d{k}:{len(v)}/{sum(v)/len(v):.0f}" for k, v in sorted(b.items())))
    r = collections.defaultdict(list)
    for dd, it, st, rs in rows:
        r[rs].append(it)
    print("  restored", {k: (len(v), round(sum(v) / len(v), 1)) for k, v in r.items()})

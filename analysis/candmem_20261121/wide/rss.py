"""old と new の最大 RSS (max_rss_mb) の比べ: 問題・種の対ごとの new/old の比の幾何平均と、差の大きい問題。使い方: python rss.py res_*.jsonl"""
import json, math, sys
from collections import defaultdict
by = defaultdict(dict)
for f in sys.argv[1:]:
    for l in open(f):
        if l.strip():
            r = json.loads(l)
            if r.get("max_rss_mb"):
                by[(r["problem"], r["seed"])][r["cfg"]] = r["max_rss_mb"]
pairs = [(k, d["old"], d["new"]) for k, d in by.items() if "old" in d and "new" in d]
if pairs:
    g = math.exp(sum(math.log(n / o) for _, o, n in pairs) / len(pairs))
    print(f"対 {len(pairs)}: 最大 RSS の new/old の幾何平均 {g:.3f}、old の合計 {sum(o for _, o, _ in pairs):.0f} MB、new の合計 {sum(n for _, _, n in pairs):.0f} MB")
    for k, o, n in sorted(pairs, key=lambda t: t[2] - t[1])[:10]:
        print(f"  減った {k}: {o} -> {n} MB")
    for k, o, n in sorted(pairs, key=lambda t: t[1] - t[2])[:5]:
        print(f"  増えた {k}: {o} -> {n} MB")

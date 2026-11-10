"""MIP の畳める数の集計。python summ_count.py <mip_count_*.jsonl>"""
import json, re, sys
for f in sys.argv[1:]:
    hit = []
    for l in open(f):
        r = json.loads(l)
        for x in r["fold"]:
            m = re.search(r"n (\d+) -> (\d+), m (\d+) -> (\d+) \(ints (\d+) -> (\d+), int groups (\d+) with (\d+) cols", x)
            if m:
                hit.append((r["name"],) + tuple(map(int, m.groups())))
    print(f, "lines", sum(1 for _ in open(f)), "folded", len(hit), "with int groups", sum(1 for h in hit if h[7] > 0))
    for h in sorted(hit, key=lambda h: (h[2] + h[4]) / (h[1] + h[3])):
        print("  %-32s n %7d -> %7d  m %7d -> %7d  ints %7d -> %7d  groups %d (%d cols)" % h)

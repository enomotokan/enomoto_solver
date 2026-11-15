"""count_*.jsonl を集計する (analysis/reflsym_20261112.md)。python summ.py count_a.jsonl count_b.jsonl ..."""
import json, re, sys
rows = []
for fn in sys.argv[1:]:
    for l in open(fn):
        r = json.loads(l)
        for ln in r["lines"]:
            m = re.match(r"REFLSYMCOUNT (.*?): n (\d+) m (\d+) bins (\d+) reflectable (\d+) \| perm: gens (\d+) orbits (\d+) moved (\d+) \| signed: gens (\d+) orbits (\d+) moved (\d+) gens_with_reflection (\d+) self_reflected (\d+) \(([\d.]+)s, ([\d.]+)s\)", ln)
            if m:
                g = m.groups()
                rows.append((r["name"], g[0], *map(int, g[1:13]), float(g[13]), float(g[14])))
        if not any(ln.startswith("REFLSYMCOUNT") for ln in r["lines"]):
            print("線なし", r["name"], r["lines"][:1], r["wall"])
print(f"{len(rows)} 行")
inc = [r for r in rows if r[11] > r[8] or r[13] > 0]
print(f"置換あり {sum(r[8] > 0 for r in rows)}、符号つきあり {sum(r[9+1-1+1] > 0 for r in rows)}、反転を含む生成元あり {sum(r[13] > 0 for r in rows)}、動く列が増えた {sum(r[11] > r[8] for r in rows)}")
print("問題 場所 n | 置換 gens orbits moved | 符号 gens orbits moved 反転gens 自己反転 | 秒")
for r in sorted(rows, key=lambda r: -(r[11] - r[8])):
    if r[6] > 0 or r[9] > 0:
        print(r[0], r[1], r[2], "|", r[6], r[7], r[8], "|", r[9], r[10], r[11], r[12], r[13], "|", r[14], r[15])

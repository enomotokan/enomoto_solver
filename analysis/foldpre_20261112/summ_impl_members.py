"""count_impl.jsonl (riNc + IMPLDIAG) から、畳みの段ごとに暗黙の整数の連続列が失われた・混ざった問題を挙げる (analysis/foldpre_20261112.md)。"""
import json, re, sys
tot = {"kept": 0, "lost": 0, "mixed": 0}
hits = []
nprob = 0
for l in open(sys.argv[1]):
    r = json.loads(l)
    nprob += 1
    st = 0
    for ln in r["fold"]:
        m = re.search(r"members: .* kept (\d+), lost (\d+) \(with cost (\d+)\), mixed with plain continuous (\d+), merged groups (\d+)", ln)
        if m:
            st += 1
            k, lo, lc, mx, mg = map(int, m.groups())
            tot["kept"] += k; tot["lost"] += lo; tot["mixed"] += mx
            if lo or mx:
                hits.append((r["name"], st, k, lo, lc, mx, mg))
print(f"{nprob} 問、段の合計: 保たれた {tot['kept']}、失われた {tot['lost']}、混ざった {tot['mixed']}")
print("問題 段 | 保たれた 失われた (費用あり) 混ざった まとめた組")
for h in hits:
    print(*h)

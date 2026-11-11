"""(3) の数え (count3.jsonl、drcr_20261110/run.py の ncnt・cnt) を集計する。MIP_FOLD_COUNT の行を 場所 (ノードの深さ・サブ MIP・再スタート) ごとに
まとめ、畳めた回数と縮み (畳んだ列 / 残った自由な列) を出す。ncnt は畳まない元の問題 (MIP_FOLD=0)、cnt は既定 (畳んだ問題の上で数える)。
使い方: python summ_count3.py count3.jsonl"""
import json, re, sys
from collections import defaultdict
pat = re.compile(r"MIP_FOLD_COUNT (.+?): free n (\d+) m (\d+) \(of n (\d+)\) -> (?:n (\d+) m (\d+) \(int groups (\d+) with (\d+) cols|no fold)")
rows = [json.loads(l) for l in open(sys.argv[1])]
for cfg in ("ncnt", "cnt"):
    print(f"## {cfg}")
    print("| 問題 | 根 (d=0) | ノード d>0 畳めた/回 (列の縮み 中央) | サブ MIP 畳めた/回 (縮み 中央) | 再スタート (1 回目以降) 畳めた/回 |")
    print("|---|---|---|---|---|")
    tot = defaultdict(lambda: [0, 0])
    for r in rows:
        if r["cfg"] != cfg:
            continue
        g = defaultdict(list)
        for l in r["fold"]:
            m = pat.match(l)
            if not m:
                continue
            tag = m.group(1); n = int(m.group(2))
            kind = "d0" if tag == "node d=0" else "node" if tag.startswith("node") else "submip" if tag.startswith("submip") else "restart"
            g[kind].append(None if m.group(5) is None else int(m.group(5)) / max(n, 1))
        def cell(k):
            v = g.get(k, [])
            if not v:
                return "—"
            f = sorted(x for x in v if x is not None)
            tot[k][0] += len(f); tot[k][1] += len(v)
            return f"{len(f)}/{len(v)}" + (f" ({f[len(f) // 2]:.2f})" if f else "")
        print(f"| {r['problem']} | {cell('d0')} | {cell('node')} | {cell('submip')} | {cell('restart')} |")
    print("合計 (畳めた/回):", {k: f"{a}/{b}" for k, (a, b) in tot.items()})

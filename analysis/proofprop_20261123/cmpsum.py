"""cmp_<札>.jsonl をまとめる: python cmpsum.py 札 → 問題・スレッドごとに old/new の時間 (回ごと)・ノード・比"""
import json, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
R = defaultdict(lambda: defaultdict(list))
for r in map(json.loads, open(D / f"cmp_{sys.argv[1]}.jsonl")):
    R[(r["problem"], r["threads"])][r["build"]].append((r["result"].get("time"), r["result"].get("nodes"), r["result"].get("status")))
print("| 問題 | スレッド | old 秒 (ノード) | new 秒 (ノード) | new/old (時間の和) |")
print("|---|---:|---|---|---:|")
for (p, th), v in sorted(R.items()):
    f = lambda xs: "・".join(f"{t:.1f} ({n})" for t, n, s in xs)
    so, sn = sum(t for t, _, _ in v["old"]), sum(t for t, _, _ in v["new"])
    print(f"| {p} | {th} | {f(v['old'])} | {f(v['new'])} | {sn / so:.3f} |" if so and sn else f"| {p} | {th} | {f(v['old'])} | {f(v['new'])} | |")

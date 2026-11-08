"""run1.py の jsonl から、問題・設定・回ごとに結果 (状態・秒・ノード・目的値・下界) と、ノードの LP の計数 (全スレッドの和、種類ごと) を表にする。
使い方: python summ.py ファイル.jsonl [kinds]   kinds を付けると種類ごとの行も出す"""
import json, re, sys
from collections import defaultdict
rows = [json.loads(l) for l in open(sys.argv[1]) if l.strip()]
kinds = len(sys.argv) > 2
print("| 問題 | 設定 | rep | 状態 | 秒 | ノード | 目的値 | 下界 | ノードの LP | 反復/LP | ms/LP | 行 | カットの行 | 出発の主実行不能 |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
for r in rows:
    agg = defaultdict(lambda: [0.0] * 8)
    for l in r.get("nodelp", []):
        m = re.match(r"MIP: nodelp tid (\d+) (\w+): n (\d+) iters ([\d.]+)/LP ms ([\d.]+)/LP rows ([\d.]+) cutrows ([\d.]+) warm \d+ prim_inf ([\d.]+)", l)
        if not m:
            continue
        k = m.group(2); n = int(m.group(3))
        for key in (k, "all"):
            a = agg[key]
            a[0] += n; a[1] += n * float(m.group(4)); a[2] += n * float(m.group(5)); a[3] += n * float(m.group(6)); a[4] += n * float(m.group(7)); a[5] += n * float(m.group(8))
    res = r["result"]
    def line(key):
        a = agg[key]; n = max(a[0], 1)
        return f"{int(a[0])} | {a[1]/n:.1f} | {a[2]/n:.2f} | {a[3]/n:.0f} | {a[4]/n:.0f} | {a[5]/n:.1f}"
    ob = res.get("objective"); bd = res.get("bound")
    print(f"| {r['problem']} | {r['cfg']} | {r['rep']} | {res.get('status')} | {res.get('time', 0) or 0:.1f} | {res.get('nodes')} | {ob if ob is None else round(ob, 4)} | {bd if bd is None else round(bd, 4)} | {line('all')} |")
    if kinds:
        for k in ("dive", "sibling", "queue", "pool", "resolve"):
            if agg[k][0]:
                print(f"| | ({k}) | | | | | | | {line(k)} |")

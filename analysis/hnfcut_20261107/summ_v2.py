"""改良 (11-08) の比べの表: v2_avoca300.jsonl・v2_rococo600.jsonl・v2_flugpl.jsonl を問題 × 設定 × 回で並べる。"""
import json, sys
from collections import defaultdict
ORDER = ["def", "hcut_b", "hcut_bc", "hcut_bcp", "hcut_abcp", "clc", "hcut_b_clc", "hcut_bc_clc"]
d = defaultdict(dict)
for f in sys.argv[1:]:
    for l in open(f):
        r = json.loads(l); d[r["problem"]][(r["cfg"], r["rep"])] = r
print("| 問題 | 設定 | 回 | 状態 | 秒 | 解 | 下界 | ギャップ | ノード | 根のカットの後 |")
print("|---|---|---|---|---|---|---|---|---|---|")
for p, v in d.items():
    for (c, s), r in sorted(v.items(), key=lambda kv: (ORDER.index(kv[0][0]) if kv[0][0] in ORDER else 99, kv[0][1])):
        x = r["result"]; o, b = x.get("objective"), x.get("bound")
        gap = f"{100 * (o - b) / abs(o):.1f}%" if o is not None and b is not None and o != 0 else "-"
        ra = r["info"].get("root_after_cuts")
        print(f"| {p} | {c} | {s} | {x.get('status')} | {x.get('time', 0):.1f} | {o} | {b if b is None else round(b, 1)} | {gap} | {x.get('nodes')} | {ra if ra is None else round(ra, 1)} |")

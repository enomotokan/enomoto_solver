"""期限の超過の比べ (ov 用): 時間切れの回の超過 (wall − tl と、結果の time − tl) を設定ごとに数える。python overrun.py res_*.jsonl"""
import json, sys
from collections import defaultdict
by = defaultdict(list)
for f in sys.argv[1:]:
    for l in open(f):
        if l.strip():
            r = json.loads(l); x = r["result"]
            if x.get("status") == "time_limit": by[r["cfg"]].append((r["problem"], r["seed"], (x.get("time") or 0) - r["tl"], (r.get("wall") or 0) - r["tl"]))
for c, v in sorted(by.items()):
    o = sorted(x[2] for x in v); w = sorted(x[3] for x in v)
    print(f"{c}: 時間切れ {len(v)} 回、結果の時刻の超過 平均 {sum(o)/len(o):.2f} 秒・最大 {o[-1]:.1f} 秒・1 秒超 {sum(x > 1 for x in o)} 回、"
          f"壁時計の超過 (起動・読み込みを含む) 平均 {sum(w)/len(w):.2f} 秒・最大 {w[-1]:.1f} 秒")
    for p, s, a, b in sorted(v, key=lambda x: -x[2])[:5]: print(f"   {p} 種 {s}: {a:.1f} 秒 (壁時計 {b:.1f})")

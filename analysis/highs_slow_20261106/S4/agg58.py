"""bench58.jsonl の集計: 設定・回ごとの解けた数と SGM (シフト 10 秒の幾何平均、時間切れは 60 秒)、問題ごとに 1.5 倍以上・10 秒以上違うもの。"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
R = defaultdict(dict)
for l in (D / "bench58.jsonl").read_text().splitlines():
    r = json.loads(l); x = r["result"]
    ok = str(x.get("status")).lower() == "optimal"
    R[(r["cfg"], r["rep"])][r["problem"]] = (x.get("time", 60.0) if ok else 60.0, ok, x.get("s4", {}))
for k, v in sorted(R.items()):
    sgm = math.exp(sum(math.log(t + 10) for t, _, _ in v.values()) / len(v)) - 10
    fired = sorted(p for p, (_, _, s) in v.items() if any(s.values()))
    print(f"{k[0]} 回 {k[1]}: {len(v)} 問, 解けた {sum(o for _, o, _ in v.values())}, SGM {sgm:.2f} 秒, 試作が働いた問題 {fired}")
cfgs = sorted({c for c, _ in R})
for c in cfgs:
    if c == "def": continue
    for rep in sorted({r for _, r in R}):
        a, b = R.get(("def", rep), {}), R.get((c, rep), {})
        for p in sorted(set(a) & set(b)):
            ta, tb = a[p][0], b[p][0]
            if max(ta, tb) >= 10 and max(ta, tb) / max(min(ta, tb), 1e-9) >= 1.5:
                print(f"  回 {rep} {p}: def {ta:.1f}{'' if a[p][1] else '(切れ)'} / {c} {tb:.1f}{'' if b[p][1] else '(切れ)'}")

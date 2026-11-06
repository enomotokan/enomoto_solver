"""bench58.jsonl の集計: 回ごと・設定ごとの解けた数と SGM (シフト 10 秒、解けなければ 60 秒)、差の大きい問題。"""
import json, math, collections, sys
from pathlib import Path
R = collections.defaultdict(dict)
for l in (Path(__file__).parent / "bench58.jsonl").read_text().splitlines():
    r = json.loads(l); x = r["result"]
    t = x.get("time") if x.get("status") == "optimal" else None
    R[r["rep"]][(r["problem"], r["cfg"])] = (min(t, 60.0) if t is not None else 60.0, t is not None, x.get("status"), x.get("objective"))
for rep, d in sorted(R.items()):
    probs = sorted({p for p, c in d if (p, "def") in d and (p, "trforce") in d})
    for c in ("def", "trforce"):
        ts = [d[(p, c)][0] for p in probs]
        print(f"rep {rep} {c}: {len(probs)} 問, 解けた {sum(d[(p, c)][1] for p in probs)}, SGM {math.exp(sum(math.log(t + 10) for t in ts) / len(ts)) - 10:.2f}")
    for p in probs:
        a, b = d[(p, "def")], d[(p, "trforce")]
        if abs(a[0] - b[0]) > 0.25 * max(a[0], b[0]) and max(a[0], b[0]) > 5 or a[1] != b[1]:
            print(f"  {p}: def {a[0]:.1f} ({a[2]}) trforce {b[0]:.1f} ({b[2]})")
        if a[3] is not None and b[3] is not None and a[1] and b[1] and abs(a[3] - b[3]) > 1e-6 * max(1, abs(a[3])):
            print(f"  !! {p}: 目的値が食い違う {a[3]} {b[3]}")

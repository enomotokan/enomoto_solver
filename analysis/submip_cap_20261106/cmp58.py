"""bench58.jsonl の比べ: 回ごとの解けた数・シフト幾何平均 (シフト 10 秒、解けない回は 60 秒)、1.5 倍以上かつ 3 秒以上違う問題。
使い方: python cmp58.py 基準 比べる"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
a, b = sys.argv[1], sys.argv[2]
R = defaultdict(dict)
for l in (Path(__file__).parent / __import__("os").environ.get("BENCH58_OUT", "bench58.jsonl")).read_text().splitlines():
    r = json.loads(l)
    st = r["result"].get("status", "")
    t = r["result"]["time"] if st.lower() == "optimal" and r["result"].get("time") is not None else 60.0
    R[r["rep"]][(r["problem"], r["cfg"])] = (t, st.lower() == "optimal", r["result"].get("objective"))
sgm = lambda ts: math.exp(sum(math.log(t + 10) for t in ts) / len(ts)) - 10
for rep in sorted(R):
    probs = sorted({p for p, c in R[rep] if (p, a) in R[rep] and (p, b) in R[rep]})
    if not probs:
        continue
    ta = [R[rep][(p, a)][0] for p in probs]; tb = [R[rep][(p, b)][0] for p in probs]
    na = sum(R[rep][(p, a)][1] for p in probs); nb = sum(R[rep][(p, b)][1] for p in probs)
    print(f"rep {rep}: {len(probs)} 問, {a} {sgm(ta):.2f} 秒・{na} 問, {b} {sgm(tb):.2f} 秒・{nb} 問 ({100 * (sgm(tb) / sgm(ta) - 1):+.1f}%)")
    for p in probs:
        x, y = R[rep][(p, a)], R[rep][(p, b)]
        if max(x[0], y[0]) >= 1.5 * min(x[0], y[0]) and abs(x[0] - y[0]) >= 3 or x[1] != y[1]:
            print(f"   {p}: {a} {x[0]:.1f}{'' if x[1] else ' (解けず)'} / {b} {y[0]:.1f}{'' if y[1] else ' (解けず)'}")
        if x[1] and y[1] and x[2] is not None and y[2] is not None and abs(x[2] - y[2]) > 1e-6 * max(1, abs(x[2])):
            print(f"   !! {p}: 最適値が違う {x[2]} / {y[2]}")

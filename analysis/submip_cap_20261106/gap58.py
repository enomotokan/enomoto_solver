"""bench58.jsonl の主ギャップの比べ: 問題ごとに全設定・全回の最良の目的値を基準に、主ギャップ |z − 最良| / max(|z|, |最良|) (解なしは 1) を出し、
回ごとの和と、設定の間で 0.01 以上違う問題を出す。最小化の問題だけを仮定する (MIPLIB は目的値の向きを揃えて解く)。
使い方: python gap58.py 基準 比べる ..."""
import json, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for l in (D / __import__("os").environ.get("BENCH58_OUT", "bench58.jsonl")).read_text().splitlines() if l.strip()]
best = {}
for r in rows:
    z = r["result"].get("objective")
    if z is not None:
        best[r["problem"]] = min(best.get(r["problem"], z), z)
def gap(r):
    z = r["result"].get("objective")
    if z is None:
        return 1.0
    b = best[r["problem"]]
    d = max(abs(z), abs(b))
    return 0.0 if d < 1e-9 else abs(z - b) / d
G = defaultdict(dict)
for r in rows:
    G[(r["rep"], r["problem"])][r["cfg"]] = gap(r)
cfgs = sys.argv[1:]
for rep in sorted({k[0] for k in G}):
    ps = [p for (rr, p), v in G.items() if rr == rep and all(c in v for c in cfgs)]
    print(f"rep {rep}: {len(ps)} 問, " + ", ".join(f"{c} {sum(G[(rep, p)][c] for p in ps):.3f}" for c in cfgs))
    for p in sorted(ps):
        v = [G[(rep, p)][c] for c in cfgs]
        if max(v) - min(v) >= 0.01:
            print(f"   {p}: " + " / ".join(f"{c} {x:.3f}" for c, x in zip(cfgs, v)))

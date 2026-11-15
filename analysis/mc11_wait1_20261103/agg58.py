"""bench58.jsonl の集計 (設定ごとの解けた数・SGM(+10)、片方だけ解けた・1.5 倍以上違った問題)。使い方: python agg58.py [基準] [設定,...]"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for l in open(D / "bench58.jsonl")]
t = defaultdict(dict)
for r in rows:
    res = r["result"]; ok = res.get("status") == "optimal"
    t[r["cfg"]][r["problem"]] = (ok, min(res.get("time") or 60, 60) if ok else 60.0)
base = sys.argv[1] if len(sys.argv) > 1 else "def"
cfgs = sys.argv[2].split(",") if len(sys.argv) > 2 else list(t)
print("| 設定 | 問題 | 解けた | SGM(+10) | 既定比 |\n|---|---|---|---|---|")
sg0 = None
for c in cfgs:
    d = t[c]; sg = math.exp(sum(math.log(v[1] + 10) for v in d.values()) / len(d)) - 10
    sg0 = sg0 or (sg if c == base else None)
    print(f"| {c} | {len(d)} | {sum(v[0] for v in d.values())} | {sg:.2f} | {'' if c == base or sg0 is None else f'{100*(sg/sg0-1):+.1f}%'} |")
print()
print("| 問題 | " + " | ".join(cfgs) + " |")
print("|---" * (len(cfgs) + 1) + "|")
for p in sorted(t[base]):
    a = [t[c][p] for c in cfgs]
    if len(set(x[0] for x in a)) > 1 or max(x[1] for x in a) > 1.5 * min(x[1] for x in a) and max(x[1] for x in a) > 2:
        print(f"| {p} | " + " | ".join(f"{x[1]:.1f}" if x[0] else "時間切れ" for x in a) + " |")

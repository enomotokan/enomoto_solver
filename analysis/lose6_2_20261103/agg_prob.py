"""res.jsonl の担当 2 問の集計 (設定ごとの解けた数・時間・SGM(+1)、時間切れは 60 秒)。回 100 番台 (当たりつけ) は除く。使い方: python agg_prob.py"""
import json, math
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
t = defaultdict(list)
for l in (D / "res.jsonl").read_text().splitlines():
    r = json.loads(l)
    if r["run"] >= 100: continue
    ok = r["status"].lower() == "optimal"
    t[(r["problem"], r["cfg"])].append((ok, min(r["time"], 60.0) if ok else 60.0))
print("| 問題 | 設定 | 解けた | 時間 (秒、時間切れは T) | SGM(+1) |\n|---|---|---|---|---|")
for (p, c), v in sorted(t.items()):
    sg = math.exp(sum(math.log(x[1] + 1) for x in v) / len(v)) - 1
    print(f"| {p} | {c} | {sum(x[0] for x in v)}/{len(v)} | {', '.join(f'{x[1]:.1f}' if x[0] else 'T' for x in v)} | {sg:.1f} |")

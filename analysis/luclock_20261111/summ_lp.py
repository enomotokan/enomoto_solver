"""LUCLOCK の LP 単体の集計 (lpbench.py の出力): 設定ごとの秒の合計・反復の幾何平均・分解の数の合計と、形ごとの揺れ
(閾値を ±1/16 動かした 3 回の反復の 最大/最小 の幾何平均)。使い方: python summ_lp.py lp.jsonl"""
import json, sys, math
from collections import defaultdict
rows = [json.loads(l) for l in open(sys.argv[1]) if l.strip()]
R = {}
for r in rows:
    lc = max(r["luclock"], key=lambda x: x["iters"]) if r["luclock"] else None
    R[(r["problem"], r["cfg"])] = (lc["iters"] if lc else 0, r["result"].get("time", float("nan")), lc["builds"] if lc else 0, r["result"].get("status"))
FAM = {"def": ["def", "f15", "f17"], "g": ["g", "g09", "g11"]}
probs = list(dict.fromkeys(p for p, _ in R))
for k, f in FAM.items():
    sp = []
    for p in probs:
        its = [R[(p, c)][0] for c in f if (p, c) in R]
        sp.append(math.log(max(its) / max(min(its), 1)))
    print(f"{k}: 揺れの幾何平均 {math.exp(sum(sp) / len(sp)):.3f}")
    for c in f:
        v = [R[(p, c)] for p in probs if (p, c) in R]
        print(f"  {c}: 秒の合計 {sum(x[1] for x in v):.1f}、反復の幾何平均 {math.exp(sum(math.log(max(x[0], 1)) for x in v) / len(v)):.0f}、分解の合計 {sum(x[2] for x in v)}、"
              f"最適 {sum(x[3] == 'optimal' for x in v)}/{len(v)}")

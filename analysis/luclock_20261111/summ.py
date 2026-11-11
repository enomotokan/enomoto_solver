"""LUCLOCK の重い根の LP の集計: 問題ごとに設定別の根の LP (最初の種類 4) の反復・秒・分解の数、
形ごとの揺れ (閾値を ±1/16 動かした 3 回の反復の 最大/最小)。使い方: python summ.py heavy.jsonl"""
import json, sys, math
from collections import defaultdict
rows = [json.loads(l) for l in open(sys.argv[1]) if l.strip()]
R = {}
for r in rows:
    rl = [x for x in r["rootlp3"] if x["kind"] == 4]
    if not rl:
        continue
    x = rl[0]
    # 根の LP の LUCLOCK の行 (反復が 1 違い以内、主ループの反復は根の LP の反復 - 1 のことがある)
    lc = [y for y in r["luclock_top"] if abs(y["iters"] - x["iters"]) <= 1]
    R[(r["problem"], r["cfg"])] = (x["iters"], x["secs"], lc[0]["builds"] if lc else None, r["result"].get("status"), r["result"].get("time"))
FAM = {"def": ["def", "f15", "f17"], "b": ["b", "bf15", "bf17"], "g": ["g", "g09", "g11"]}
probs = list(dict.fromkeys(p for p, _ in R))
print("| 問題 | " + " | ".join(f"{c} 反復・秒・分解" for f in FAM.values() for c in f) + " | 揺れ def | 揺れ b | 揺れ g |")
print("|---" * (2 + 9 + 2) + "|")
spread = defaultdict(list); tsum = defaultdict(float); isum = defaultdict(float)
for p in probs:
    cells, sp = [], []
    for k, f in FAM.items():
        its = []
        for c in f:
            v = R.get((p, c))
            cells.append(f"{v[0]}・{v[1]:.2f}・{v[2]}" if v else "-")
            if v:
                its.append(v[0]); tsum[c] += v[1]; isum[c] += math.log(max(v[0], 1))
        s = max(its) / max(min(its), 1) if its else float("nan")
        sp.append(f"{s:.2f}"); spread[k].append(math.log(s))
    print(f"| {p} | " + " | ".join(cells) + " | " + " | ".join(sp) + " |")
print()
for k in FAM:
    print(f"{k}: 揺れ (最大/最小) の幾何平均 {math.exp(sum(spread[k]) / len(spread[k])):.3f}; ",
          "秒の合計 " + ", ".join(f"{c} {tsum[c]:.1f}" for c in FAM[k]), "; 反復の幾何平均 " + ", ".join(f"{c} {math.exp(isum[c] / len(probs)):.0f}" for c in FAM[k]))

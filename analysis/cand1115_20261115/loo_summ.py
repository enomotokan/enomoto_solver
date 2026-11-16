"""1 つ外し (res_L_s*.jsonl) の集計: 問題ごとに、各設定の時間 (shift 1 秒、時間切れは 60 秒) の幾何平均を cand1115 との比で出す。
比が 1 より小さい = その項目を外すと速い = その項目が遅らせている。python loo_summ.py > loo_summ.txt"""
import json, glob, math, collections
from loo_cfg import CFGS, PROBS
R = collections.defaultdict(dict)
for f in glob.glob("res_L_s*.jsonl"):
    for l in open(f):
        r = json.loads(l); res = r["result"]
        t = min(res["time"], 60) if res.get("status") in ("optimal", "infeasible") else 60
        R[(r["problem"], r["cfg"])][r["seed"]] = t
names = list(CFGS)
print("問題 | " + " | ".join(names))
allr = collections.defaultdict(list)
for p in PROBS:
    base = R[(p, "cand1115")]
    row = []
    for c in names:
        v = R[(p, c)]
        ks = [s for s in v if s in base]
        lr = [math.log((v[s] + 1) / (base[s] + 1)) for s in ks]
        g = math.exp(sum(lr) / len(lr)) if lr else float("nan")
        allr[c] += lr
        tm = math.exp(sum(math.log(v[s] + 1) for s in ks) / len(ks)) - 1 if ks else float("nan")
        row.append(f"{g:.2f} ({tm:.1f}s)" if c != "cand1115" else f"{tm:.1f}s")
    print(p, "|", " | ".join(row))
print("7 問まとめ (cand1115 比):", {c: round(math.exp(sum(v) / len(v)), 3) for c, v in allr.items() if v})

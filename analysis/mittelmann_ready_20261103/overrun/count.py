# bench14〜20 の jsonl から、60 秒の上限を 1 秒以上超えた回を問題・設定ごとに数える
import json, glob, collections, sys
fs = sorted(glob.glob("analysis/halve_submip_20261030/bench1[4-9]_s*.jsonl") + glob.glob("analysis/halve_submip_20261030/bench20_s*.jsonl"))
tot = collections.Counter(); over = collections.Counter(); mx = collections.defaultdict(float); n=0; nover=0
byb = collections.Counter(); bybo = collections.Counter(); allt=[]
for f in fs:
    b = f.split("/")[-1].split("_")[0]
    for l in open(f):
        r = json.loads(l); t = r["result"].get("time")
        if t is None: continue
        n += 1; k = (r["problem"], r["cfg"]); tot[k] += 1; byb[b] += 1
        if t >= 61.0:
            over[k] += 1; nover += 1; bybo[b] += 1; mx[k] = max(mx[k], t); allt.append(t)
print(f"files={len(fs)} runs={n} over={nover}")
for b in sorted(byb): print(b, byb[b], bybo[b])
pc = collections.Counter(); pt = collections.Counter(); pm = collections.defaultdict(float)
for (p,c),v in over.items(): pc[p]+=v; pm[p]=max(pm[p],mx[(p,c)])
for (p,c),v in tot.items(): pt[p]+=v
print("\n| 問題 | 超過回数 / 回数 | 最大 (秒) | 設定ごと (超過/回数) |")
print("|---|---|---|---|")
for p,v in pc.most_common():
    cs = ", ".join(f"{c} {over[(p,c)]}/{tot[(p,c)]}" for c in sorted({c for (q,c) in tot if q==p}) if over[(p,c)])
    print(f"| {p} | {v} / {pt[p]} | {pm[p]:.1f} | {cs} |")
import statistics
print("\n超過の分布: ", sorted(round(x,1) for x in allt))

"""13 問の設定ごとの時間 (種 1・2、時間切れは TL に * )・幾何平均の比 (def に対して、+1 秒のずらし)・ノード・根の下界。
使い方: python table.py main.jsonl where.jsonl ... > table.md"""
import json, math, sys
R = {}
for f in sys.argv[1:]:
    for l in open(f):
        if not l.strip():
            continue
        r = json.loads(l); res = r["result"]
        t = res["time"] if res.get("status") == "optimal" else r["tl"]
        R.setdefault(r["problem"], {}).setdefault(r["cfg"], {})[r["rep"]] = (t, res.get("status") == "optimal", res.get("nodes"), res.get("root_bound"))
CF = ["def", "zh", "zhroot", "zhtree", "zhr", "zht"]
cfs = [c for c in CF if any(c in d for d in R.values())]
print("| 問題 | " + " | ".join(cfs) + " |")
print("|---|" + "---|" * len(cfs))
gm = {c: [] for c in cfs}
for p, d in R.items():
    cells = []
    for c in cfs:
        if c not in d:
            cells.append("-"); continue
        ts = " / ".join(f"{d[c][s][0]:.0f}{'' if d[c][s][1] else '*'}" for s in sorted(d[c]))
        g = math.exp(sum(math.log(d[c][s][0] + 1) for s in d[c]) / len(d[c]))
        gd = math.exp(sum(math.log(d["def"][s][0] + 1) for s in d["def"]) / len(d["def"]))
        cells.append(ts + ("" if c == "def" else f" ({g / gd:.2f})"))
        if p != "enlight_hard" and len(d[c]) == 2:
            gm[c].append((p, math.log(g / gd)))
    print(f"| {p} | " + " | ".join(cells) + " |")
print()
for c in cfs[1:]:
    xs = gm[c]
    if xs:
        print(f"- {c}: def に対する幾何平均の比 {math.exp(sum(x for _, x in xs) / len(xs)):.3f} (2 種そろった {len(xs)} 問)")

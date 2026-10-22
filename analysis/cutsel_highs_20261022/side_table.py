"""副作用の一覧: python3 side_table.py logs/batch5.out  (問題 × 設定、回ごとに 解けた時間 / 時間切れの暫定解と下界)"""
import json, sys, collections
d = collections.defaultdict(list)
for l in open(sys.argv[1]):
    p, cfg, n, js = l.split(" ", 3)
    r = json.loads(js)
    if r["status"] == "optimal":
        s = f"opt {r['time']:.1f}s"
    else:
        o = r["objective"]; b = r["best_bound"]
        s = f"TL {o if o is None else round(o, 2)} / {b if b is None else round(b, 2)}"
    d[(p, cfg)].append(s + f" (root {r['root_bound'] if r['root_bound'] is None else round(r['root_bound'], 2)})")
probs = list(dict.fromkeys(k[0] for k in d))
cfgs = list(dict.fromkeys(k[1] for k in d))
print("| 問題 | " + " | ".join(cfgs) + " |")
print("|---|" + "---|" * len(cfgs))
for p in probs:
    print(f"| {p} | " + " | ".join("<br>".join(d[(p, c)]) for c in cfgs) + " |")

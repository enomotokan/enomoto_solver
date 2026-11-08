"""改良 (11-08) の表: python table_imp.py file.jsonl [設定 ...]  (問題 × 種 で 状態・時間・暫定解・下界)"""
import json, sys
rs = [json.loads(l) for l in open(sys.argv[1])]
cfgs = sys.argv[2:] or list(dict.fromkeys(r["cfg"] for r in rs))
probs = list(dict.fromkeys(r["problem"] for r in rs))
print("| 問題 | 種 | " + " | ".join(cfgs) + " |")
print("|---|---|" + "---|" * len(cfgs))
for p in probs:
    for s in sorted(set(r["rep"] for r in rs if r["problem"] == p)):
        cells = []
        for c in cfgs:
            r = next((r for r in rs if r["problem"] == p and r["cfg"] == c and r["rep"] == s), None)
            if r is None:
                cells.append("-"); continue
            x = r["result"]
            if x.get("status") == "optimal":
                cells.append(f"**最適 {x['time']:.0f} 秒**")
            else:
                o, b = x.get("objective"), x.get("bound")
                cells.append(f"{'-' if o is None else round(o, 1)} / {'-' if b is None else round(b, 1)}")
        print(f"| {p} | {s} | " + " | ".join(cells) + " |")

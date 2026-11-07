"""long600.jsonl を問題ごとに (設定 × 種) の 暫定解 / 下界 / ノード の表にする"""
import json, sys
rs = [json.loads(l) for l in open(sys.argv[1] if len(sys.argv) > 1 else "long600.jsonl")]
probs = list(dict.fromkeys(r["problem"] for r in rs))
cfgs = ["def", "hnf", "sub"]
print("| 問題 | 種 | " + " | ".join(f"{c} 暫定解 / 下界 / ノード" for c in cfgs) + " |")
print("|---|---|" + "---|" * len(cfgs))
for p in probs:
    for s in (1, 2):
        cells = []
        for c in cfgs:
            r = next((r for r in rs if r["problem"] == p and r["cfg"] == c and r["rep"] == s), None)
            if r is None:
                cells.append("-"); continue
            x = r["result"]
            o = x.get("objective"); b = x.get("bound")
            cells.append(f"{o if o is None else round(o, 1)} / {b if b is None else round(b, 1)} / {x.get('nodes')}" + ("" if x.get("status") == "time_limit" else f" ({x.get('status')} {x.get('time'):.0f}s)"))
        print(f"| {p} | {s} | " + " | ".join(cells) + " |")

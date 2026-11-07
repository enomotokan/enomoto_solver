"""長い計測 (long.jsonl) の表: 問題・回ごとに def・mc・mk の結果 (状態・時間・暫定解・下界・ノード) と根の最初のループの下界。使い方: python summ_long.py"""
import json
from pathlib import Path
D = Path(__file__).resolve().parent
R = {}
for l in open(D / "long.jsonl"):
    r = json.loads(l); R.setdefault((r["problem"], r["rep"]), {})[r["cfg"]] = r
print("| 問題 | 回 | " + " | ".join(["def", "mc", "mk"]) + " |")
print("|---|---|---|---|---|")
for (p, rep), d in sorted(R.items()):
    cells = []
    for c in ["def", "mc", "mk"]:
        r = d.get(c)
        if not r:
            cells.append("-"); continue
        x = r["result"]; rl = r.get("root_loops") or [{}]
        st = f"最適 {x['time']:.0f} 秒" if x.get("status") == "optimal" else f"時間切れ (解 {x.get('objective'):.6g}・下界 {x.get('bound'):.6g})"
        cells.append(f"{st}、根 {rl[0].get('obj1', float('nan')):.6g}")
    print(f"| {p} | {rep} | " + " | ".join(cells) + " |")

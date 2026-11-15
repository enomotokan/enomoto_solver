"""Mittelmann 600 秒の比べ (res.jsonl の m600def・m600fixh): 問題ごとに種ごとの状態・秒・暫定解・下界を並べる。
使い方: python m600.py"""
import json
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
R = defaultdict(dict)
order = []
for r in map(json.loads, (D / "res.jsonl").read_text().splitlines()):
    if not r["tag"].startswith("m600"):
        continue
    if r["problem"] not in order:
        order.append(r["problem"])
    R[r["problem"]][(r["tag"][4:], r["seed"])] = r["result"]
def cell(x):
    if x is None:
        return "-"
    z, b = x.get("objective"), x.get("bound")
    if x["status"] == "optimal":
        return f"最適 {x['time']:.0f} 秒 ({z:.8g})"
    return f"{z:.8g} (下界 {b:.6g})" if z is not None else f"解なし (下界 {b:.6g})"
print("| 問題 | 種 | 既定 | fixh |")
print("|---|---|---|---|")
for p in order:
    for s in sorted({k[1] for k in R[p]}):
        print(f"| {p} | {s} | {cell(R[p].get(('def', s)))} | {cell(R[p].get(('fixh', s)))} |")

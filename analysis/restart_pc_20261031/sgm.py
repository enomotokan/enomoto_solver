"""損の 4 問・得の 5 問の SGM (シフト 1 秒、時間切れは 60 秒) と解けた数 (res.jsonl、回 0〜2)。使い方: python sgm.py"""
import json, math
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
LOSS = ["binkar10_1", "swath1", "mik-250-20-75-4", "dcmulti"]
GAIN = ["h80x6320d", "qiu", "misc07", "nu25-pr12", "physiciansched6-2"]
CFGS = ["def", "hs_pi0_bgw", "hs_pi0_bgw_r", "hs_pi0_bgw_pc", "hs_pir_bgw_pc"]
t = defaultdict(list)
for l in open(D / "res.jsonl"):
    r = json.loads(l)
    if r["rep"] >= 90:
        continue
    o = json.loads(r["out"]) if r["out"].startswith("{") else {}
    t[(r["prob"], r["cfg"])].append(o["time"] if o.get("status") == "optimal" else None)
print("| 設定 | 損の 4 問 SGM (解けた / 回) | 得の 5 問 SGM (解けた / 回) |")
print("|---|---|---|")
for c in CFGS:
    out = []
    for g in (LOSS, GAIN):
        v = [x for p in g for x in t[(p, c)]]
        s = math.exp(sum(math.log(min(x, 60.0) + 1 if x is not None else 61.0) for x in v) / len(v)) - 1
        out.append(f"{s:.1f} ({sum(x is not None for x in v)}/{len(v)})")
    print(f"| {c} | " + " | ".join(out) + " |")

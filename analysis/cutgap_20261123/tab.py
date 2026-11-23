"""ours.jsonl・scip.jsonl・highs.jsonl から問題ごとの根の下界の表を出す: python tab.py"""
import json
from pathlib import Path
D = Path(__file__).resolve().parent
O, S, H = {}, {}, {}
for l in open(D / "ours.jsonl"):
    r = json.loads(l); O[(r["problem"], r["tag"])] = r
for l in open(D / "scip.jsonl"):
    r = json.loads(l)
    if "table" in r and r["off"] == "-" and not r["extra"]: S[r["problem"]] = r
for l in open(D / "highs.jsonl"):
    r = json.loads(l)
    if r["tag"] in ("def", "wp"): H[r["problem"]] = r
f = lambda x: "—" if x is None else f"{float(x):.6g}"
print("| 問題 | 当方 根の LP | 当方 カット後 | SCIP 根の LP | SCIP カット後 | SCIP 根の終わり | HiGHS 最初の LP | HiGHS カット後 | 当方 (HiGHS の前処理後) 根の LP | 同 カット後 |")
print("|---|---|---|---|---|---|---|---|---|---|")
for p in sorted({k[0] for k in O}):
    o = O.get((p, "def")); hp = O.get((p, "hpre")); s = S.get(p); h = H.get(p)
    hr = [x for x in (h or {}).get("root", []) if x[1] not in ("-inf", "inf")]
    print(f"| {p} | {f(o and (o.get('root_lp') or [None])[0])} | {f(o and o['result'][4] if o and o.get('result') else None)} | {f(s and s['first_lp'])} | {f(s and s['after_cuts'])} | {f(s and s['root_dual'])} | {f(hr[0][1] if hr else None)} | {f(hr[-1][1] if hr else None)} | {f(hp and (hp.get('root_lp') or [None])[0])} | {f(hp and hp['result'][4] if hp and hp.get('result') else None)} |")

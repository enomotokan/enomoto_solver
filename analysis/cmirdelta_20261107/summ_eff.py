"""効く問題の確かめの集計 (eff.jsonl の種 3・4 と bench58.jsonl の種 1・2)。使い方: python summ_eff.py > eff_summary.txt
2 回目 (絞った形 cdn と同じ wheel の def): python summ_eff.py n > effn_summary.txt (effn.jsonl だけを読む)"""
import json, math
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
rows = []
import sys
FILES = ["effn.jsonl"] if sys.argv[1:] == ["n"] else ["bench58.jsonl", "eff.jsonl"]
for f in FILES:
    if (D / f).exists():
        rows += [json.loads(l) for l in open(D / f)]
probs = ["neos-3083819-nubu", "neos-911970", "hypothyroid-k1", "p200x1188c", "sp150x300d", "h80x6320d", "qnet1", "drayage-25-23"]
cfgs = ["def", "cd", "cdc", "cdl", "cdn"]
T = defaultdict(dict)
S = defaultdict(dict)
for r in rows:
    if r["problem"] in probs:
        res = r["result"]
        t = res.get("time") if res.get("status") == "optimal" and res.get("time", 99) <= 60 else 60.0
        T[(r["problem"], r["cfg"])][r["rep"]] = t
        S[(r["problem"], r["cfg"])][r["rep"]] = (res.get("root_bound"), (r.get("sep") or {}).get("chosen_new"), (r.get("sep") or {}).get("chosen_all"))
print("時間 (種 1 2 3 4、未解決は 60) と幾何平均 (10 秒ずらし)")
for p in probs:
    for c in cfgs:
        d = T.get((p, c), {})
        if not d:
            continue
        ts = [d.get(k) for k in (1, 2, 3, 4)]
        v = [x for x in ts if x is not None]
        g = math.exp(sum(math.log(x + 10) for x in v) / len(v)) - 10
        ch = " ".join(f"{int(s[1] or 0)}/{int(s[2] or 0)}" for k, s in sorted(S[(p, c)].items()) if s[2])
        print(f"{p:20s} {c:4s} " + " ".join(f"{x:6.1f}" if x is not None else "     -" for x in ts) + f"  sgm {g:6.1f}  根の下界 " +
              " ".join(f"{S[(p, c)][k][0]:.6g}" for k in sorted(S[(p, c)]) if S[(p, c)][k][0] is not None) + (f"  新しい δ のカット/全体 {ch}" if ch else ""))

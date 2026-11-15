"""58 問の根の 1・2 回目 (root58.jsonl の def・mk、種 1・2) を並べ、どれかが違う問題を出す (既定どうしの揺れも)。使い方: python noise_root.py"""
import json
from pathlib import Path
D = Path(__file__).resolve().parent
R = {}
for l in open(D / "root58.jsonl"):
    r = json.loads(l)
    if r["cfg"] in ("def", "mk"):
        R.setdefault(r["problem"], {})[(r["cfg"], r.get("rep", 1))] = r["result"].get("bound")
print("問題 / def 種1 種2 / mk 種1 種2 / 判定 (両方の回で mk が def の良い方を超える: 上、悪い方を下回る: 下)")
for p, d in R.items():
    try:
        a, b, c, e = d[("def", 1)], d[("def", 2)], d[("mk", 1)], d[("mk", 2)]
    except KeyError:
        continue
    if None in (a, b, c, e):
        continue
    s = 1e-6 * max(1, abs(a))
    if max(abs(a - b), abs(c - a), abs(e - b)) <= s:
        continue
    j = "上" if min(c, e) > max(a, b) + s else "下" if max(c, e) < min(a, b) - s else ""
    print(f"{p:22s} def {a:12.7g} {b:12.7g}  mk {c:12.7g} {e:12.7g}  {'def揺れ' if abs(a - b) > s else '':6s} {j}")

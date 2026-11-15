"""58 問の根 (root58.jsonl) の def・mk・mk50 (種 1・2) を並べ、2 回とも既定の良い方を超える (上) / 悪い方を下回る (下) 問題を出す。
使い方: python noise_root50.py"""
import json
from pathlib import Path
D = Path(__file__).resolve().parent
R = {}
for l in open(D / "root58.jsonl"):
    r = json.loads(l)
    if r["cfg"] in ("def", "mk", "mk50"):
        R.setdefault(r["problem"], {})[(r["cfg"], r.get("rep", 1))] = r["result"].get("bound")
def judge(d, c, s):
    a, b, x, y = d[("def", 1)], d[("def", 2)], d[(c, 1)], d[(c, 2)]
    return "上" if min(x, y) > max(a, b) + s else "下" if max(x, y) < min(a, b) - s else ""
cnt = {"mk": [0, 0], "mk50": [0, 0]}
print("問題 / def 種1 種2 / mk 種1 種2 / mk50 種1 種2 / mk 判定 / mk50 判定")
for p, d in R.items():
    keys = [(c, r) for c in ("def", "mk", "mk50") for r in (1, 2)]
    if any(d.get(k) is None for k in keys):
        continue
    s = 1e-6 * max(1, abs(d[("def", 1)]))
    j1, j2 = judge(d, "mk", s), judge(d, "mk50", s)
    for c, j in (("mk", j1), ("mk50", j2)):
        if j: cnt[c][0 if j == "上" else 1] += 1
    if j1 or j2:
        print(f"{p:22s} " + " ".join(f"{d[k]:12.7g}" for k in keys) + f"  {j1:2s} {j2:2s}")
print("2 回とも上 / 下:", {c: v for c, v in cnt.items()})

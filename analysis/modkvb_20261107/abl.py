"""5 章: どの部分が効くか (根、種 1・2)。58 問は def・mk50 (pyw4)、Mittelmann は def5・mk50b (pyw5)。各列は「種 1 / 種 2」。
判定: 2 回とも既定の良い方を超える = 上、悪い方を下回る = 下 (最小化の下界なので大きいほど良い)。使い方: python abl.py"""
import json
from pathlib import Path
D = Path(__file__).resolve().parent
def load(f):
    R = {}
    for l in open(D / f):
        r = json.loads(l); R.setdefault(r["problem"], {})[(r["cfg"], r.get("rep", 1))] = r["result"].get("bound")
    return R
P58 = ["misc07", "fiber", "mzzv42z", "p0201", "nu25-pr12", "swath1", "rout", "blend2", "mc11", "h80x6320d", "qnet1", "neos5"]
PM = ["comp21-2idx", "comp07-2idx", "enlight_hard", "mushroom-best", "mzzv11", "neos-3024952-loue", "neos-3046615-murg", "milo-v12-6-r2-40-1"]
cols = ["nozh", "mk50", "mk50nozh", "mk50novb", "mk50nocut", "mk50k2"]
def judge(d, dc, c):
    try:
        a, b, x, y = d[(dc, 1)], d[(dc, 2)], d[(c, 1)], d[(c, 2)]
    except KeyError:
        return "?"
    if None in (a, b, x, y): return "?"
    s = 1e-6 * max(1, abs(a))
    return "上" if min(x, y) > max(a, b) + s else "下" if max(x, y) < min(a, b) - s else "="
print("| 問題 | 既定 | " + " | ".join(cols) + " |")
print("|---|---|" + "---|" * len(cols))
for f, P, dc, mk in [("root58.jsonl", P58, "def", "mk50"), ("rootmitt.jsonl", PM, "def5", "mk50b")]:
    R = load(f)
    for p in P:
        d = R.get(p, {})
        g = lambda c: "/".join(f"{d.get((c, r)):.6g}" if d.get((c, r)) is not None else "-" for r in (1, 2))
        cells = []
        for c in cols:
            cc = mk if c == "mk50" else c
            cells.append(f"{g(cc)} {judge(d, dc, cc)}")
        print(f"| {p} | {g(dc)} | " + " | ".join(cells) + " |")

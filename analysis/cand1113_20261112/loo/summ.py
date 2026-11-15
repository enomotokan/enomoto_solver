"""res_L_s*.jsonl を問題 × 設定の表にする。時間は種 1・2 の幾何平均 (時間切れは 60 秒として数える)。
括弧は def に対する比、* は時間切れの数。"""
import json, math
from pathlib import Path
from cfg import CFGS
D = Path(__file__).resolve().parent
R = {}
for f in sorted(D.glob("res_L_s*.jsonl")):
    for l in f.read_text().splitlines():
        r = json.loads(l); res = r["result"]
        t = res.get("time") if res.get("status") == "optimal" else 60.0
        R[(r["problem"], r["cfg"], r["seed"])] = (min(t or 60.0, 60.0), res.get("status"))
P = ["neos-960392", "binkar10_1", "roll3000", "pk1", "10teams", "neos8", "h80x6320d"]
def gm(p, c):
    v = [R[(p, c, s)][0] for s in (1, 2) if (p, c, s) in R]
    return math.exp(sum(math.log(max(x, 1.0)) for x in v) / len(v)) if v else None
print("設定 \\ 問題 | " + " | ".join(P))
print("---|" + "---|" * len(P))
for c in CFGS:
    row = []
    for p in P:
        g, d = gm(p, c), gm(p, "def")
        cell = "-" if g is None else f"{g:.1f} ({g / d:.2f})"
        n_tl = sum(1 for s in (1, 2) if R.get((p, c, s), (0, ""))[1] == "time_limit")
        row.append(cell + ("*" * n_tl))
    print(f"{c} | " + " | ".join(row))
print("\n種ごと (秒、種 1 / 種 2、TL は時間切れ):\n")
print("設定 \\ 問題 | " + " | ".join(P))
print("---|" + "---|" * len(P))
def one(p, c, s):
    if (p, c, s) not in R: return "-"
    t, st = R[(p, c, s)]
    return "TL" if st == "time_limit" else f"{t:.1f}"
for c in CFGS:
    print(f"{c} | " + " | ".join(f"{one(p, c, 1)} / {one(p, c, 2)}" for p in P))

"""bench58.jsonl の要約: 回ごとの解けた数・幾何平均 (秒 +1、時間切れは 60)、差の大きい問題、係数の幅 (fix の notes)。"""
import json, math, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
f = sys.argv[1] if len(sys.argv) > 1 else "bench58.jsonl"
rs = [json.loads(l) for l in (D / f).read_text().splitlines()]
by = {}
for r in rs:
    by.setdefault(r["rep"], {}).setdefault(r["problem"], {})[r["cfg"]] = r
def t(r):
    res = r["result"]
    return res.get("time") if res.get("status") == "optimal" else 60.0
rng = {}
for r in rs:
    for n in r.get("notes", []):
        m = re.search(r"scaled twin for (\d+) rows .*range ([0-9.e+]+)", n)
        if m and r["problem"] not in rng:
            rng[r["problem"]] = (int(m.group(1)), float(m.group(2)))
for rep, d in sorted(by.items()):
    ps = [p for p, v in d.items() if "base" in v and "fix" in v]
    for c in ("base", "fix"):
        sol = sum(d[p][c]["result"].get("status") == "optimal" for p in ps)
        gm = math.exp(sum(math.log(t(d[p][c]) + 1) for p in ps) / len(ps)) - 1
        print(f"回 {rep} {c}: 問題 {len(ps)} 解けた {sol} 幾何平均 {gm:.2f}")
    for p in ps:
        a, b = t(d[p]["base"]), t(d[p]["fix"])
        if abs(a - b) > 0.25 * max(a, b) and max(a, b) > 5:
            print(f"   {p}: base {a:.1f} fix {b:.1f}  rows/range {rng.get(p)}")
print("係数の幅 1e4 以上:", sorted((p, v) for p, v in rng.items() if v[1] >= 1e4))
print("双子を作らなかった問題:", sorted(set(r["problem"] for r in rs) - set(rng)))

"""run.py の結果の表: python analysis/objprobehull_20261124/tab.py res_a.jsonl res_b.jsonl ...
(問題, 種) ごとに既定 (def) と試作 (hull) の状態・時間・ノード・根の下界、試作の締め付け (最初の根の打ち切り値つきプロービングの行) を並べる。"""
import json, re, sys
from collections import defaultdict
R = defaultdict(dict)
for f in sys.argv[1:]:
    for l in open(f):
        r = json.loads(l); R[(r["problem"], r["seed"])][r["cfg"]] = r
def hull_info(r):
    ls = [l for l in r["log"] if "probing hull" in l]
    tot = [0, 0, 0]
    for l in ls:
        m = re.search(r"(\d+) bounds tightened on (\d+) columns.*?(\d+) integer columns fixed", l)
        for k in range(3): tot[k] += int(m.group(k + 1))
    return f"{len(ls)} 回 境界 {tot[0]} 列 {tot[1]} 固定 {tot[2]}"
def fixed_info(r):
    return sum(int(m.group(1)) for l in r["log"] for m in [re.search(r"probed \(\d+ passes\), (\d+) fixed", l)] if m)
def rst(r):
    return sum(1 for l in r["log"] if "restart check" in l)
print("| 問題 | 種 | 既定: 状態 / 時間 / ノード / 根の下界 / 試しの固定 | 試作: 状態 / 時間 / ノード / 根の下界 / 試しの固定 | 試作の締め付け (合計) |")
print("|---|---|---|---|---|")
for (p, s), d in sorted(R.items()):
    if "def" not in d or "hull" not in d: continue
    cells = []
    for c in ("def", "hull"):
        x = d[c]["result"]
        cells.append(f"{x.get('status')} / {x.get('time', 0):.1f} / {x.get('nodes')} / {x.get('root_bound')} / {fixed_info(d[c])}")
    print(f"| {p} | {s} | {cells[0]} | {cells[1]} | {hull_info(d['hull'])} |")

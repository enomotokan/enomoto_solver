"""run.py の結果を設定ごとに並べる: python analysis/objprobehull_20261124/tab2.py 札,札,... res...jsonl
(問題, 見切り) ごとに、各設定の状態・時間・ノード・根の下界 (種ごと) と、og の締め付け (根の回の合計)、hull2 の関係の本数・木の中で締めた回数を出す。
最後に、全部の設定がそろった回のずらし幾何平均 (ずらし 10 秒、解けない回は上限の秒) の比 (各設定 / 最初の設定) を見切りごとに出す。"""
import json, math, re, sys
from collections import defaultdict
cfgs = sys.argv[1].split(",")
R = defaultdict(dict)
for f in sys.argv[2:]:
    for l in open(f):
        r = json.loads(l)
        b = "外す" if r.get("base") else "既定"
        R[(r["problem"], b, r["seed"])][r["cfg"]] = r
def extra(r):
    c = r["cfg"]; lg = r["log"]
    if c == "og":
        fx = sum(int(m.group(2)) for l in lg for m in [re.search(r"OBJPROP_GLOBAL: (\d+) integer bounds tightened globally, (\d+)", l)] if m)
        pr = any("ROOT_OBJPROP_GLOBAL proved" in l for l in lg)
        return f"固定 {fx}" + (" 証明" if pr else "")
    if c == "hull2":
        rel = [int(m.group(1)) for l in lg for m in [re.search(r"=2: (\d+) bound implications added", l)] if m]
        tr = [m.groups() for l in lg for m in [re.search(r"from x_j to y (\d+), contrapositive \(y to x_j\) (\d+)", l)] if m]
        return f"関係 {sum(rel)} 木 {tr[-1] if tr else ('0', '0')}"
    if c == "hull":
        return ""
    return ""
def t(r):
    x = r["result"]
    return x.get("time", r["tl"]) if x.get("status") == "optimal" else r["tl"]
print("| 問題 | 見切り | 種 | " + " | ".join(f"{c}: 時間 / ノード / 根の下界" for c in cfgs) + " | 補足 |")
print("|---|---|---|" + "---|" * len(cfgs) + "---|")
agg = defaultdict(lambda: defaultdict(list))
for (p, b, s), d in sorted(R.items()):
    if not all(c in d for c in cfgs):
        continue
    cells = []
    for c in cfgs:
        x = d[c]["result"]
        rb = x.get("root_bound")
        st = "" if x.get("status") == "optimal" else f"{x.get('status')} "
        cells.append(f"{st}{t(d[c]):.1f} / {x.get('nodes')} / {rb:.6g}" if rb is not None else f"{st}{t(d[c]):.1f} / {x.get('nodes')} / -")
        agg[b][c].append(t(d[c]))
    ex = "; ".join(f"{c}: {extra(d[c])}" for c in cfgs if extra(d[c]))
    print(f"| {p} | {b} | {s} | " + " | ".join(cells) + f" | {ex} |")
for b, a in agg.items():
    g = {c: math.exp(sum(math.log(v + 10) for v in a[c]) / len(a[c])) for c in cfgs}
    print(f"\n見切り {b}: {len(a[cfgs[0]])} 回、ずらし幾何平均の比 " + ", ".join(f"{c}/{cfgs[0]} {g[c] / g[cfgs[0]]:.3f}" for c in cfgs[1:]))

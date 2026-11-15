"""res58.jsonl (58 問・60 秒・4 スレッド) の表: python -I summ58.py"""
import json, math
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
res = defaultdict(dict)
for l in open(D / "res58.jsonl"):
    e = json.loads(l)
    res[(e["problem"], e["rep"])][e["cfg"]] = e["result"]
cfgs = ["def", "c1"]
ok = lambda r: r.get("status") in ("optimal", "Optimal")
keys = [k for k in res if all(c in res[k] for c in cfgs)]
def gap(r):
    o, b = r.get("objective"), r.get("bound")
    if o is None or b is None: return 1.0
    return min(1.0, abs(o - b) / max(abs(o), abs(b), 1e-9))
print("| 設定 | 解けた数 | SGM (ずらし 10 秒) | 主双対ギャップの和 | 最初の解の時刻の幾何平均 (ずらし 1 秒) | 解なし | WARNING |")
print("|---|---|---|---|---|---|---|")
for c in cfgs:
    rs = [res[k][c] for k in keys]
    t = [min(r["time"], 60.0) if ok(r) and r.get("time") is not None else 60.0 for r in rs]
    sgm = math.exp(sum(math.log(x + 10) for x in t) / len(t)) - 10
    ft = [r.get("first_solution_time") if r.get("first_solution_time") is not None else 60.0 for r in rs]
    fg = math.exp(sum(math.log(x + 1) for x in ft) / len(ft)) - 1
    print(f"| {c} | {sum(ok(r) for r in rs)} / {len(rs)} | {sgm:.2f} | {sum(gap(r) for r in rs):.3f} | {fg:.3f} | {sum(r.get('objective') is None for r in rs)} | {sum(r.get('mip_warnings', 0) for r in rs)} |")
print()
print("| 問題 | 既定 状態 | 既定 時間 | 既定 暫定解 | 既定 最初の解 (秒) | 試作 状態 | 試作 時間 | 試作 暫定解 | 試作 最初の解 (秒) |")
print("|---|---|---|---|---|---|---|---|---|")
f = lambda x: "-" if x is None else f"{x:.8g}"
for k in sorted(keys):
    a, b = res[k]["def"], res[k]["c1"]
    diff = ok(a) != ok(b) or (not ok(a) and a.get("objective") != b.get("objective")) or (ok(a) and abs(a["time"] - b["time"]) > max(5, 0.3 * a["time"]))
    mark = " ◆" if diff else ""
    print(f"| {k[0]}{mark} | {a.get('status')} | {f(a.get('time'))[:6]} | {f(a.get('objective'))} | {f(a.get('first_solution_time'))[:6]} | {b.get('status')} | {f(b.get('time'))[:6]} | {f(b.get('objective'))} | {f(b.get('first_solution_time'))[:6]} |")
print("\n◆: 片方だけ解けた・両方時間切れで暫定解が違う・解けた時間の差が 5 秒かつ 3 割を超える")

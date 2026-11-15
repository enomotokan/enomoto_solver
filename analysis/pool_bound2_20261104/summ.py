"""bench.jsonl (58 問、def・h・gik4) の集計 (2026-11-04、memcap_c_20261104/summ.py の写し。その元は uccase12_mem1_20261103/summ.py の写し)。解けた = optimal かつ 60 秒以内、SGM はずらし 10 秒 (解けない回は 60 秒)。
HiGHS の最適値 (analysis/highs58_20261030/highs.jsonl) との食い違いも数える。使い方: python summ.py"""
import json, math, statistics, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
R, X = {}, {}
for l in open(D / "bench.jsonl"):
    r = json.loads(l); R.setdefault(r["problem"], {})[r["cfg"]] = r["result"]; X.setdefault(r["problem"], {})[r["cfg"]] = r
H = {json.loads(l)["problem"]: json.loads(l)["result"] for l in open(WT / "analysis" / "highs58_20261030" / "highs.jsonl")}
ok = lambda r: r.get("status") == "optimal" and r.get("time", 99) <= 60.0
tm = lambda r: r["time"] if ok(r) else 60.0
sgm = lambda xs: math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
def bad(p, r):
    h = H.get(p, {})
    if r.get("status") != "optimal" or h.get("status") != "optimal": return False
    return abs(r["objective"] - h["objective"]) > 1e-6 * max(1, abs(h["objective"]))
CF = ["def", "h", "gik4"]
ps = [p for p in PROBS if all(c in R.get(p, {}) for c in CF)]
print(f"問題数 {len(ps)}\n")
print("| 設定 | 解けた | SGM (秒) | crash | HiGHS の最適値との食い違い | 最大 RSS MB (中央値/最大) |")
print("|---|---|---|---|---|---|")
for c in CF:
    rs = [R[p][c] for p in ps]; xs = [X[p][c]["max_rss_kb"] / 1024 for p in ps]
    print(f"| {c} | {sum(map(ok, rs))} | {sgm([tm(r) for r in rs]):.2f} | {sum(r.get('status') in ('crash', 'killed') for r in rs)} | "
          f"{sum(bad(p, R[p][c]) for p in ps)} | {statistics.median(xs):.0f} / {max(xs):.0f} |")
print("\n解けたかが違う問題・時間が 1.5 倍以上違う問題 (どちらかが 5 秒以上)・最大 RSS が 200 MB 以上違う問題:\n")
print("| 問題 | " + " | ".join(CF) + " | " + " | ".join(f"{c} RSS MB" for c in CF) + " |")
print("|---" * (2 * len(CF) + 1) + "|")
f = lambda r: f"{r.get('status')} {r.get('time', 0):.1f}s" + ("" if ok(r) else f" ({r.get('objective')})")
for p in ps:
    rs = [R[p][c] for c in CF]; rss = [X[p][c]["max_rss_kb"] / 1024 for c in CF]; ts = [tm(r) for r in rs]
    if len({ok(r) for r in rs}) > 1 or (max(ts) >= 5 and max(ts) / max(min(ts), 1e-3) >= 1.5) or max(rss) - min(rss) >= 200:
        print(f"| {p} | " + " | ".join(map(f, rs)) + " | " + " | ".join(f"{x:.0f}" for x in rss) + " |")

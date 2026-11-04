"""pool_bound1 の集計 (2026-11-04、memcap_c_20261104/summ.py の写しを 2 つの表に広げた)。
使い方: python summ.py [<設定,...> (既定 def,worst)]
- long.jsonl (5 問・600 秒): 問題ごとの状態・目的値・下界・時間・ノード数・最大 RSS。
- bench.jsonl (58 問・60 秒): 解けた = optimal かつ 60 秒以内、SGM はずらし 10 秒 (解けない回は 60 秒)、HiGHS の最適値との食い違い。"""
import json, math, statistics, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
CF = sys.argv[1].split(",") if len(sys.argv) > 1 else ["def", "worst"]


def load(name):
    R, X = {}, {}
    f = D / name
    if not f.exists():
        return R, X
    for l in open(f):
        r = json.loads(l); R.setdefault(r["problem"], {})[r["cfg"]] = r["result"]; X.setdefault(r["problem"], {})[r["cfg"]] = r
    return R, X


R, X = load("long.jsonl")
if R:
    print("## 5 問・600 秒\n")
    print("| 問題 | 設定 | 状態 | 目的値 | 下界 | 時間 (秒) | ノード | 最大 RSS MB |")
    print("|---|---|---|---|---|---|---|---|")
    for p in R:
        for c in CF:
            if c in R[p]:
                r = R[p][c]
                print(f"| {p} | {c} | {r.get('status')} | {r.get('objective')} | {r.get('bound')} | {r.get('time', 0):.1f} | {r.get('nodes')} | {X[p][c]['max_rss_kb'] / 1024:.0f} |")
R, X = load("bench.jsonl")
if R:
    H = {json.loads(l)["problem"]: json.loads(l)["result"] for l in open(WT / "analysis" / "highs58_20261030" / "highs.jsonl")}
    ok = lambda r: r.get("status") == "optimal" and r.get("time", 99) <= 60.0
    tm = lambda r: r["time"] if ok(r) else 60.0
    sgm = lambda xs: math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10

    def bad(p, r):
        h = H.get(p, {})
        if r.get("status") != "optimal" or h.get("status") != "optimal":
            return False
        return abs(r["objective"] - h["objective"]) > 1e-6 * max(1, abs(h["objective"]))
    CF = [c for c in CF if any(c in R[p] for p in R)]
    ps = [p for p in PROBS if all(c in R.get(p, {}) for c in CF)]
    print(f"\n## 58 問・60 秒 (問題数 {len(ps)})\n")
    print("| 設定 | 解けた | SGM (秒) | crash | HiGHS の最適値との食い違い | 最大 RSS MB (中央値/最大) |")
    print("|---|---|---|---|---|---|")
    for c in CF:
        rs = [R[p][c] for p in ps]; xs = [X[p][c]["max_rss_kb"] / 1024 for p in ps]
        print(f"| {c} | {sum(map(ok, rs))} | {sgm([tm(r) for r in rs]):.2f} | {sum(r.get('status') in ('crash', 'killed') for r in rs)} | "
              f"{sum(bad(p, R[p][c]) for p in ps)} | {statistics.median(xs):.0f} / {max(xs):.0f} |")
    a0, b0 = CF[0], CF[1]
    print(f"\n解けたかが違う問題・時間が 1.5 倍以上違う問題 (どちらかが 5 秒以上)・最大 RSS が 200 MB 以上違う問題:\n")
    print(f"| 問題 | {a0} | {b0} | {a0} RSS MB | {b0} RSS MB |")
    print("|---|---|---|---|---|")
    for p in ps:
        a, b = R[p][a0], R[p][b0]; ra, rb = X[p][a0]["max_rss_kb"] / 1024, X[p][b0]["max_rss_kb"] / 1024
        ta, tb = tm(a), tm(b)
        if ok(a) != ok(b) or (max(ta, tb) >= 5 and max(ta, tb) / max(min(ta, tb), 1e-3) >= 1.5) or abs(ra - rb) >= 200:
            f = lambda r: f"{r.get('status')} {r.get('time', 0):.1f}s" + ("" if ok(r) else f" ({r.get('objective')})")
            print(f"| {p} | {f(a)} | {f(b)} | {ra:.0f} | {rb:.0f} |")

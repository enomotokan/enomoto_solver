"""bench58.jsonl の集計 (def と memb): SGM (ずらし 10 秒、60 秒を超えたら時間切れ)・解けた数・最大 RSS・片方だけ解けた問題・時間の比の大きい問題。"""
import json, math
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
R = defaultdict(dict)
for l in open(D / "bench58.jsonl"):
    r = json.loads(l); R[r["problem"]][r["cfg"]] = r["result"]
def ok(x): return x.get("status") == "optimal" and x.get("time", 99) <= 60.0
def t(x): return x["time"] if ok(x) else 60.0
def sgm(xs): return math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
ks = [p for p in R if "def" in R[p] and "memb" in R[p]]
for c in ("def", "memb"):
    xs = [R[p][c] for p in ks]
    print(f"{c:5s} n={len(ks)} SGM {sgm([t(x) for x in xs]):.2f}s 解けた {sum(map(ok, xs))} 最大 RSS の和 {sum(x.get('maxrss_mb', 0) for x in xs) / 1024:.1f} GB 最大 {max(x.get('maxrss_mb', 0) for x in xs):.0f} MB")
a, b = sgm([t(R[p]["def"]) for p in ks]), sgm([t(R[p]["memb"]) for p in ks])
print(f"比 memb/def {b / a - 1:+.1%}")
print("\n片方だけ解けた・時間の比が 1.5 倍を超える (10 秒ずらし)・異常:")
for p in ks:
    d, m = R[p]["def"], R[p]["memb"]
    r = (t(m) + 10) / (t(d) + 10)
    if ok(d) != ok(m) or r > 1.5 or r < 1 / 1.5 or d.get("status") not in ("optimal", "time_limit") or m.get("status") not in ("optimal", "time_limit"):
        f = lambda x: f"{x['time']:.1f}" if ok(x) else f"{x.get('status')}"
        print(f"  {p:28s} def {f(d):>10s} mem2 {f(m):>10s}  RSS {d.get('maxrss_mb', 0):.0f} / {m.get('maxrss_mb', 0):.0f} MB")
    if ok(d) and ok(m) and abs(d["objective"] - m["objective"]) > 1e-6 * max(1, abs(d["objective"])):
        print("  食い違い", p, d["objective"], m["objective"])
print("\nRSS の大きい問題 (どちらかが 1 GB 超):")
for p in ks:
    d, m = R[p]["def"], R[p]["memb"]
    if max(d.get("maxrss_mb", 0), m.get("maxrss_mb", 0)) > 1024:
        print(f"  {p:28s} {d.get('maxrss_mb', 0):7.0f} {m.get('maxrss_mb', 0):7.0f} MB")

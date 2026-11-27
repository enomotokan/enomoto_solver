"""M の def/rp を根の RINS の秒 (rootlp_20261120/rhscan.jsonl: 根だけ・1 スレッド・種 1・600 秒の DEBUG_RHSTAT の main の RINS 行の秒の和、1 スレッドなので 4 スレッドの根の RINS とは別のビルド・別の条件の目安) で層に分けた
ずらし幾何平均の比 (ずらし 10 秒、解けない回は TL)。使い方: python rins_M.py res_M_s*.jsonl"""
import json, math, sys
from pathlib import Path
T = 600.0
rins = {}
for l in open(Path(__file__).resolve().parents[2] / "rootlp_20261120" / "rhscan.jsonl"):
    r = json.loads(l)
    if r["set"] != "M": continue
    s = [float(x[2]) for x in r.get("rhstat") or [] if len(x) > 2 and x[0] == "main" and x[1] == "RINS"]
    rins[r["problem"]] = sum(s) if s else 0.0
by = {}
for f in sys.argv[1:]:
    for l in open(f):
        if l.strip():
            r = json.loads(l); by.setdefault(r["problem"], {})[r["cfg"]] = r["result"]
t = lambda x: min(x.get("time") or T, T) if x.get("status") == "optimal" else T
def ci(ps):
    lr = [math.log((t(by[p]["rp"]) + 10) / (t(by[p]["def"]) + 10)) for p in ps]
    n = len(lr)
    if n < 2: return f"n={n}"
    m = sum(lr) / n; sd = (sum((x - m) ** 2 for x in lr) / (n - 1)) ** .5; h = 1.96 * sd / n ** .5
    sv = sum(by[p]["def"].get("status") == "optimal" for p in ps), sum(by[p]["rp"].get("status") == "optimal" for p in ps)
    return f"n={n} 比 {math.exp(m):.3f} [{math.exp(m-h):.3f}, {math.exp(m+h):.3f}] 解けた def {sv[0]} / rp {sv[1]}"
ps = [p for p in by if "def" in by[p] and "rp" in by[p]]
known = [p for p in ps if p in rins]
print(f"rhscan に RINS の秒がある M の問題: {len(known)} / {len(ps)} (無いものは根の LP に届かなかったか走査外)")
for lo, hi in ((0, 0.3), (0.3, 1), (1, 5), (5, 1e9), (0.3, 1e9), (1, 1e9)):
    sel = [p for p in known if lo <= rins[p] < hi]
    print(f"根の RINS {lo}〜{hi if hi < 1e9 else ''} 秒: {ci(sel)}")
print("根の RINS 1 秒以上の問題 (RINS 秒, def 状態/秒, rp 状態/秒):")
for p in sorted((p for p in known if rins[p] >= 1), key=lambda p: -rins[p]):
    a, b = by[p]["def"], by[p]["rp"]
    print(f"  {p}: {rins[p]:.1f} | {a['status']} {a['time']:.1f} obj {a.get('objective')} | {b['status']} {b['time']:.1f} obj {b.get('objective')}")

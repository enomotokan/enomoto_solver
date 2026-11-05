"""vs55.sh の複数の設定の表: python -I vs_table2.py [設定 ...] (既定 def def2 d c1)。
各設定の Local-MIP の最初の解の時刻・3 秒の最良・手数を並べ、def と比べた勝ち負けを数える"""
import re, glob, os, sys, math
D = os.path.dirname(os.path.abspath(__file__))
CFGS = sys.argv[1:] or ["def", "def2", "d", "c1"]
PAT = re.compile(r"lm first (\S+) t (\S+) best (\S+) least_viol (\d+) steps (\d+)")
def get(p, c):
    f = f"{D}/logs/{p}_{c}.log"
    if not os.path.exists(f): return None
    l = next((l for l in open(f, errors="replace") if l.startswith("FJVSLM")), None)
    if not l: return None
    first, t, best, lv, st = PAT.search(l).groups()
    return dict(first=None if first == "none" else float(first), t=None if t == "-" else float(t),
                best=None if best == "none" else float(best), lv=int(lv), steps=int(st))
probs = sorted({os.path.basename(f).rsplit("_", 1)[0] for f in glob.glob(f"{D}/logs/*_def.log")})
f = lambda r: "-" if r is None else ("解なし (" + str(r["lv"]) + ")" if r["best"] is None else f"{r['best']:.6g} ({r['t']:.3f})")
print("| 問題 | " + " | ".join(f"{c} 3 秒の最良 (最初の解の秒)" for c in CFGS) + " |")
print("|---|" + "---|" * len(CFGS))
for p in probs:
    print(f"| {p} | " + " | ".join(f(get(p, c)) for c in CFGS) + " |")
def cmp(a, b):
    """b が a より良ければ 1、悪ければ -1、同じ 0"""
    if a["best"] is None and b["best"] is None: return 0
    if a["best"] is None: return 1
    if b["best"] is None: return -1
    if abs(a["best"] - b["best"]) <= 1e-6 * max(1, abs(a["best"])): return 0
    return 1 if b["best"] < a["best"] else -1
print()
print("| 設定 | 解なし | def より良い | def より悪い | 同じ | 最初の解が def より早い / 遅い (0.01 秒超) | 手数の比 (/ def) の幾何平均 |")
print("|---|---|---|---|---|---|---|")
for c in CFGS:
    rs = [(get(p, "def"), get(p, c)) for p in probs]
    rs = [(a, b) for a, b in rs if a and b]
    w = [cmp(a, b) for a, b in rs]
    fa = sum(1 for a, b in rs if a["t"] is not None and b["t"] is not None and b["t"] < a["t"] - 0.01)
    fb = sum(1 for a, b in rs if a["t"] is not None and b["t"] is not None and b["t"] > a["t"] + 0.01)
    rat = [b["steps"] / a["steps"] for a, b in rs if a["steps"] > 0 and b["steps"] > 0]
    g = math.exp(sum(map(math.log, rat)) / len(rat)) if rat else float("nan")
    print(f"| {c} | {sum(b['best'] is None for a, b in rs)} | {w.count(1)} | {w.count(-1)} | {w.count(0)} | {fa} / {fb} | {g:.3f} |")

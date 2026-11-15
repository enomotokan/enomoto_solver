"""種 3 つ (0 = 接尾辞なし、1 = _s1、2 = _s2) の 55 問の集計: python -I vs_seed.py。
既定 (def)・重複だけ除く (d)・本家並み (c1) を、同じ種の def と比べる。種 0 の def は 1 回目 (def) を使う"""
import re, glob, os, math
from collections import Counter
D = os.path.dirname(os.path.abspath(__file__))
PAT = re.compile(r"lm first (\S+) t (\S+) best (\S+) least_viol (\d+) steps (\d+)")
def get(p, c):
    f = f"{D}/logs/{p}_{c}.log"
    if not os.path.exists(f): return None
    l = next((l for l in open(f, errors="replace") if l.startswith("FJVSLM")), None)
    if not l: return None
    first, t, best, lv, st = PAT.search(l).groups()
    return dict(t=None if t == "-" else float(t), best=None if best == "none" else float(best), lv=int(lv), steps=int(st))
probs = sorted({os.path.basename(f).rsplit("_", 1)[0] for f in glob.glob(f"{D}/logs/*_def.log")})
SEEDS = ["", "_s1", "_s2"]
def cmp(a, b):
    if a["best"] is None and b["best"] is None: return 0
    if a["best"] is None: return 1
    if b["best"] is None: return -1
    if abs(a["best"] - b["best"]) <= 1e-6 * max(1, abs(a["best"])): return 0
    return 1 if b["best"] < a["best"] else -1
print("| 設定 | 種 | 解なし | def より良い | def より悪い | 同じ | 最初の解が早い / 遅い (0.01 秒超) | 手数の比の幾何平均 |")
print("|---|---|---|---|---|---|---|---|")
tot = {c: Counter() for c in ["def", "d", "c1"]}
for c in ["def", "d", "c1"]:
    for s in SEEDS:
        rs = [(get(p, "def" + s), get(p, c + s)) for p in probs]
        rs = [(a, b) for a, b in rs if a and b]
        w = [cmp(a, b) for a, b in rs]
        fa = sum(1 for a, b in rs if a["t"] is not None and b["t"] is not None and b["t"] < a["t"] - 0.01)
        fb = sum(1 for a, b in rs if a["t"] is not None and b["t"] is not None and b["t"] > a["t"] + 0.01)
        rat = [b["steps"] / a["steps"] for a, b in rs if a["steps"] > 0 and b["steps"] > 0]
        g = math.exp(sum(map(math.log, rat)) / len(rat))
        ns = sum(b["best"] is None for a, b in rs)
        tot[c].update(ns=ns, win=w.count(1), lose=w.count(-1), tie=w.count(0), fa=fa, fb=fb)
        print(f"| {c} | {s or '_s0'} | {ns} | {w.count(1)} | {w.count(-1)} | {w.count(0)} | {fa} / {fb} | {g:.3f} |")
    t = tot[c]
    print(f"| **{c}** | 計 | {t['ns']} | {t['win']} | {t['lose']} | {t['tie']} | {t['fa']} / {t['fb']} | |")
print()
print("解なしになった (問題: 3 つの種での 解なしの回数 [最少違反行])、設定ごと:")
for c in ["def", "d", "c1"]:
    out = []
    for p in probs:
        rs = [get(p, c + s) for s in SEEDS]
        k = sum(r is not None and r["best"] is None for r in rs)
        if k: out.append(f"{p} {k} [{','.join(str(r['lv']) for r in rs if r)}]")
    print(f"- {c}: " + "、".join(out))

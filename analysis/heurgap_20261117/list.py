"""一覧 (HEURGAP 1): 当方と HiGHS 1.15.1 の 600 秒の結果から、暫定解の質で当方が負ける問題を並べる。python list.py > list.txt
- 出典: analysis/vshighs_20261117/res_[MA]_s*.jsonl (0.1.9 の既定 def と highs、種 1) を優先し、ない問題は
  analysis/cand1113_20261112/res_M_s*.jsonl (cand1113 と highs、種 1) で埋める (列 src)。
- 主のギャップ = |obj − 最良既知| / max(|obj|, |最良既知|) (最良既知は scripts/miplib2017-v31.solu、なければ両者の良い方)。
- 対象: 両方時間切れ、または片方だけ最適を示した問題。HiGHS の結果には最初の解の時刻がない。"""
import glob, json, math
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
solu = {}
for l in open(WT / "scripts/miplib2017-v31.solu"):
    s = l.split()
    if len(s) >= 3 and s[0] in ("=opt=", "=best="): solu[s[1]] = float(s[2])
def load(pat, ours):
    out = {}
    for f in glob.glob(str(WT / pat)):
        for l in open(f):
            r = json.loads(l)
            if r.get("seed") != 1: continue
            if r["cfg"] in (ours, "highs"): out.setdefault(r["problem"], {})[r["cfg"] if r["cfg"] == "highs" else "ours"] = r["result"]
    return out
new = load("analysis/vshighs_20261117/res_[MA]_s*.jsonl", "def")
old = load("analysis/cand1113_20261112/res_M_s*.jsonl", "cand1113")
def opt(r): return r.get("status", "").lower() in ("optimal",)
def gap(o, b):
    if o is None or b is None or not math.isfinite(o): return math.inf
    d = max(abs(o), abs(b))
    return 0.0 if d < 1e-9 else abs(o - b) / d
rows = []
for p in sorted(set(new) | set(old)):
    src, d = ("new", new[p]) if p in new and len(new[p]) == 2 else ("old", old.get(p, {}))
    if len(d) < 2: continue
    o, h = d["ours"], d["highs"]
    if opt(o) and opt(h): continue
    oo, ho = o.get("objective"), h.get("objective")
    oo = oo if isinstance(oo, (int, float)) and abs(oo) < 1e20 else None
    ho = ho if isinstance(ho, (int, float)) and abs(ho) < 1e20 else None
    best = solu.get(p)
    if best is None:
        c = [x for x in (oo, ho) if x is not None]
        best = min(c) if c else None  # 最小化を仮定 (Mittelmann は全部最小化の形で書かれている)
    go, gh = gap(oo, best), gap(ho, best)
    rows.append((p, src, "opt" if opt(o) else "tl", "opt" if opt(h) else "tl", oo, ho, best, go, gh, o.get("first_solution_time"), o.get("time"), h.get("time")))
def f(x): return "-" if x is None else ("inf" if x == math.inf else f"{x:.4g}")
print("問題 | 出典 | 当方 | HiGHS | 当方の値 | HiGHS の値 | 最良既知 | 当方の主ギャップ | HiGHS の主ギャップ | 当方の最初の解 [秒] | 当方の時間 | HiGHS の時間")
print("\n## 当方が負ける (当方の主ギャップ > HiGHS の主ギャップ + 1e-6)、差の大きい順")
lose = sorted([r for r in rows if r[7] > r[8] + 1e-6], key=lambda r: -(min(r[7], 1) - min(r[8], 1)))
for r in lose: print(" | ".join(map(lambda x: x if isinstance(x, str) else f(x), r)))
print(f"\n## 当方が勝つか同じ ({len(rows) - len(lose)} 問)")
for r in sorted([r for r in rows if not r[7] > r[8] + 1e-6], key=lambda r: r[0]): print(" | ".join(map(lambda x: x if isinstance(x, str) else f(x), r)))

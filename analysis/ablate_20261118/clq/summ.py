"""段階 2 の A の集計: python summ.py > summ_A.txt。
問題・種ごとの秒 (時間切れは TL) を def・fb・sa で並べ、問題ごとに「def / fb」(シグモイドの遅れ) と「sa / fb」の比を出す
(比 = (t + 10) / (t_fb + 10)、どちらかが解けた種だけ)。遅い側 (irp・supportcase33) と速い側 (mzzv11・neos-933966・neos-1456979・mzzv42z) に分けて見る。
判定 (README の段階 2): 遅い側で def / fb が全種 1.25 倍超、かつ速い側で def / fb が 1 未満のときだけ、場合分けの if を作る。
食い違い (最適値の違い・最適と実行不能の混在) と crash も数える。"""
import json, glob, math, sys
from pathlib import Path
D = Path(__file__).resolve().parent
CF = ["def", "fb", "sa"]
SLOW = ["irp", "supportcase33"]; FAST = ["mzzv11", "neos-933966", "neos-1456979", "mzzv42z"]
def st(r): return str(r["result"].get("status") or "").lower()
def solved(r): return st(r) in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
rows = [json.loads(l) for f in sorted(glob.glob(str(D / "res_A_s*.jsonl"))) for l in open(f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
probs = sorted({p for p, c, s in R}); seeds = sorted({s for p, c, s in R})
crash = sum(1 for r in rows if st(r) in ("", "none", "crash", "error", "fetch_error"))
print(f"A: 行数 {len(rows)}、問題 {len(probs)}、種 {seeds}、crash {crash}")
bad = []
for p in probs:
    for s in seeds:
        rs = [R[(p, c, s)] for c in CF if (p, c, s) in R]
        objs = [r["result"].get("objective") for r in rs if st(r) == "optimal"]
        if objs and max(objs) - min(objs) > 1e-6 * max(1, abs(objs[0])): bad.append((p, s, objs))
        if {st(r) for r in rs} >= {"optimal", "infeasible"}: bad.append((p, s, "opt/inf"))
print("食い違い", len(bad), bad)
print("\n| 問題 | 種 | " + " | ".join(CF) + " |"); print("|---" * (len(CF) + 2) + "|")
for p in probs:
    for s in seeds:
        if any((p, c, s) in R for c in CF):
            print(f"| {p} | {s} | " + " | ".join((f"{t(R[(p, c, s)]):.0f}" + ("" if solved(R[(p, c, s)]) else " TL")) if (p, c, s) in R else "-" for c in CF) + " |")
def ratios(p, c):
    out = []
    for s in seeds:
        if (p, c, s) in R and (p, "fb", s) in R and (solved(R[(p, c, s)]) or solved(R[(p, "fb", s)])):
            out.append((t(R[(p, c, s)]) + 10) / (t(R[(p, "fb", s)]) + 10))
    return out
def gm(v): return math.exp(sum(map(math.log, v)) / len(v)) if v else float("nan")
for name, grp in (("遅い側", SLOW), ("速い側", FAST), ("揺れの確かめ", [p for p in probs if p not in SLOW + FAST])):
    print(f"\n== {name}")
    allv = {c: [] for c in ("def", "sa")}
    for p in grp:
        line = []
        for c in ("def", "sa"):
            v = ratios(p, c); allv[c] += v
            line.append(f"{c}/fb {gm(v):.2f} {[round(x, 2) for x in v]}")
        print(f"  {p}: " + "、".join(line))
    print("  まとめ: " + "、".join(f"{c}/fb SGM {gm(v):.3f} ({len(v)} 組)" for c, v in allv.items()))
slow_ok = all(len(ratios(p, "def")) >= 2 and all(x > 1.25 for x in ratios(p, "def")) for p in SLOW)
fast_ok = gm([x for p in FAST for x in ratios(p, "def")]) < 1.0
print(f"\n判定: 遅い側が全種 1.25 倍超 {slow_ok}、速い側で sigmoid の得 {fast_ok} → " + ("場合分けの if を作る" if slow_ok and fast_ok else "二極化なしで閉じる"))

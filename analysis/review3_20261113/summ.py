"""REVIEW3 の集計 (58 問 × 種 1〜4 × 60 秒): 各項目 / base の対の比 (shift 1 秒の SGM、時間切れは 60 秒、どちらかが解けた問題、区間は問題単位のブートストラップ 95%)、
解けた回の数、時間切れの回の終わりのギャップ (幾何平均、両方が時間切れの回)、問題ごとの秒。python summ.py > summ.txt"""
import json, glob, math, random, collections
from pathlib import Path
from cfg import CFGS
D = Path(__file__).resolve().parent
TL = 60.0
SH = 1.0
rows = [json.loads(l) for f in sorted(glob.glob(str(D / "res_P_s*.jsonl"))) for l in open(f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
def st(r): return str(r["result"].get("status", "")).lower()
def solved(r): return (st(r).startswith("optimal") or st(r).startswith("infeasible")) and (r["result"].get("time") or 1e9) <= TL + 5
def t(r): return min(r["result"]["time"], TL) if solved(r) else TL
def gap(r):
    o, b = r["result"].get("objective"), r["result"].get("bound")
    if o is None or b is None or not (math.isfinite(o) and math.isfinite(b)): return None
    return abs(o - b) / max(1e-9, abs(o), abs(b))
probs = sorted({p for p, c, s in R}); seeds = [1, 2, 3, 4]
crash = [(r["problem"], r["cfg"], r["seed"]) for r in rows if st(r) in ("crash", "error", "fetch_error", "") or "error" in r["result"]]
print(f"行数 {len(rows)} / 1160、crash など {len(crash)} {crash[:10]}")
# 食い違い: 最適同士の目的値、時間切れの下界が最適値を越える
bad = []
for p in probs:
    opt = [r["result"]["objective"] for (q, c, s), r in R.items() if q == p and st(r).startswith("optimal") and r["result"].get("objective") is not None]
    if opt and max(opt) - min(opt) > 1e-6 * max(1, abs(max(opt))): bad.append((p, "obj", min(opt), max(opt)))
print("食い違い", bad)
print()
print("| 項目 | 対の比 [95%] | 問題数 | 解けた (base → 項目) | 4 種とも 1.25 倍以上遅い | 4 種とも 0.8 倍以下 | 時間切れのギャップの比 |")
print("|---|---|---|---|---|---|---|")
for c in CFGS:
    if c == "base": continue
    logr = collections.defaultdict(list); nb = nc = 0; gl = []
    for p in probs:
        for s in seeds:
            a, b = R.get((p, "base", s)), R.get((p, c, s))
            if not a or not b: continue
            nb += solved(a); nc += solved(b)
            if solved(a) or solved(b): logr[p].append(math.log((t(b) + SH) / (t(a) + SH)))
            elif gap(a) and gap(b): gl.append(math.log((gap(b) + 1e-4) / (gap(a) + 1e-4)))
    ps = list(logr)
    if not ps: continue
    def ratio(q): v = [x for p in q for x in logr[p]]; return math.exp(sum(v) / len(v))
    random.seed(0); bs = sorted(ratio([random.choice(ps) for _ in ps]) for _ in range(4000))
    slow = [p for p in ps if len(logr[p]) == 4 and all(x > math.log(1.25) for x in logr[p])]
    fast = [p for p in ps if len(logr[p]) == 4 and all(x < math.log(0.8) for x in logr[p])]
    g = f"{math.exp(sum(gl)/len(gl)):.2f} ({len(gl)} 回)" if gl else "-"
    print(f"| {c} | {ratio(ps)-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}] | {len(ps)} | {nb} → {nc} | {'・'.join(slow) or '-'} | {'・'.join(fast) or '-'} | {g} |")
print()
print("問題ごとの秒 (種 1〜4、T(g) は時間切れで終わりのギャップ g%):")
cs = list(CFGS)
print("| 問題 | " + " | ".join(cs) + " |"); print("|---" * (len(cs) + 1) + "|")
def cell(r):
    if r is None: return "."
    if solved(r): return f"{t(r):.0f}"
    g = gap(r); return f"T({100*g:.2f})" if g is not None else "T"
for p in probs:
    print(f"| {p} | " + " | ".join("・".join(cell(R.get((p, c, s))) for s in seeds) for c in cs) + " |")

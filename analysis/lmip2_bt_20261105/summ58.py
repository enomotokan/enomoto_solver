"""run58.py の結果 (r58.jsonl) の集計: python -I summ58.py [設定名 ...] (最初が基準)
解けた数・SGM (shift 10、60 秒で打ち切り)・主ギャップの和 (参照値は solu と全設定の目的値の最良)・最初の解の時刻の幾何平均 (shift 1)・
基準と比べて解けた/解けなくなった問題・最適値の食い違い"""
import json, math, sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
WT = HERE.parents[1]
names = sys.argv[1:] or ["base", "l"]
R = {}
for l in open(HERE / "r58.jsonl", encoding="utf-8"):
    r = json.loads(l)
    R.setdefault(r["problem"], {})[r["cfg"]] = r["result"]
full = {p: v for p, v in R.items() if all(c in v for c in names)}
solu = {}
for l in open(WT / "scripts" / "miplib2017-v31.solu"):
    s = l.split()
    if len(s) >= 3 and s[0] in ("=opt=", "=best="):
        solu[s[1]] = float(s[2])
def ok(x): return x.get("status") == "optimal" and x.get("time", 99) < 60
def t(x): return min(x.get("time", 60), 60.0) if ok(x) else 60.0
def sgm(xs, sh=10): return math.exp(sum(math.log(x + sh) for x in xs) / len(xs)) - sh
def obj(x): return x.get("objective") if x.get("objective") is not None and math.isfinite(x.get("objective")) else None
def gap(z, ref):
    if z is None: return 1.0
    if abs(z - ref) <= 1e-9: return 0.0
    if z * ref < 0: return 1.0
    return min(1.0, abs(z - ref) / max(abs(z), abs(ref)))
print(f"そろった問題: {len(full)}")
print("| 設定 | 解けた | SGM (秒) | 主ギャップの和 | 最初の解の時刻の幾何平均 (秒) | 解なし |")
print("|---|---|---|---|---|---|")
for c in names:
    xs = [t(full[p][c]) for p in full]
    gs = []
    for p in full:
        vals = [obj(full[p][d]) for d in names if obj(full[p][d]) is not None] + ([solu[p]] if p in solu else [])
        ref = min(vals) if vals else None  # 最小化として扱う
        gs.append(gap(obj(full[p][c]), ref) if ref is not None else 0.0)
    fs = [full[p][c].get("first_solution_time") for p in full]
    fs = [min(f, 60.0) if f is not None else 60.0 for f in fs]
    print(f"| {c} | {sum(ok(full[p][c]) for p in full)} | {sgm(xs):.2f} | {sum(gs):.3f} | {sgm(fs, 1):.2f} | {sum(obj(full[p][c]) is None for p in full)} |")
b = names[0]
for c in names[1:]:
    print(f"\n{c}: 基準で解けず→解けた {[p for p in full if ok(full[p][c]) and not ok(full[p][b])]}、解けた→解けず {[p for p in full if not ok(full[p][c]) and ok(full[p][b])]}")
    print("時間の差が大きい問題 (どちらかが解いて、比 1.5 倍以上):")
    for p in sorted(full):
        x, y = t(full[p][b]), t(full[p][c])
        if (ok(full[p][b]) or ok(full[p][c])) and max(x, y) / max(min(x, y), 0.5) >= 1.5:
            print(f"  {p}: {b} {x:.1f} / {c} {y:.1f}")
    print("どちらも解けず、目的値が違う問題 (基準 / 設定):")
    for p in sorted(full):
        if not ok(full[p][b]) and not ok(full[p][c]):
            zb, zc = obj(full[p][b]), obj(full[p][c])
            if zb != zc:
                print(f"  {p}: {zb} / {zc}")
bad = [(p, c, v.get("objective")) for p, d in full.items() for c, v in d.items() if c in names and
       ((ok(v) and p in solu and abs(v["objective"] - solu[p]) > 1e-6 * max(1, abs(solu[p])) + 1e-6) or v.get("status") in ("crash", "error"))]
print("\n最適値の食い違い・異常終了:", bad)

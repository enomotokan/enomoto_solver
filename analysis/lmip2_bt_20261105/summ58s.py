"""種を変えた 58 問の計測 (r58s.jsonl、base_s<k>・l_s<k>) の集計: python -I summ58s.py
種ごとと、種を合わせた (問題, 種) で、解けた数・SGM (shift 10)・主ギャップの和・最初の解の時刻の幾何平均 (shift 1)・解なしを出す。
主ギャップの参照値は solu と全部の回の目的値の最良 (最小化として扱う)。問題ごとに、種を合わせた時間の SGM の比が大きいものも出す。"""
import json, math
from pathlib import Path
HERE = Path(__file__).resolve().parent
WT = HERE.parents[1]
R = {}
for l in open(HERE / "r58s.jsonl", encoding="utf-8"):
    r = json.loads(l)
    cfg, sd = r["cfg"].rsplit("_s", 1)
    R.setdefault((r["problem"], int(sd)), {})[cfg] = r["result"]
full = {k: v for k, v in R.items() if "base" in v and "l" in v}
seeds = sorted({k[1] for k in full})
solu = {}
for l in open(WT / "scripts" / "miplib2017-v31.solu"):
    s = l.split()
    if len(s) >= 3 and s[0] in ("=opt=", "=best="):
        solu[s[1]] = float(s[2])
def ok(x): return x.get("status") == "optimal" and x.get("time", 99) < 60
def t(x): return min(x.get("time", 60), 60.0) if ok(x) else 60.0
def sgm(xs, sh=10): return math.exp(sum(math.log(x + sh) for x in xs) / len(xs)) - sh
def obj(x):
    z = x.get("objective")
    return z if z is not None and math.isfinite(z) else None
ref = {}
for (p, sd), d in full.items():
    for v in d.values():
        z = obj(v)
        if z is not None: ref[p] = min(ref.get(p, z), z)
for p in {k[0] for k in full}:
    if p in solu: ref[p] = min(ref.get(p, solu[p]), solu[p])
def gap(z, r):
    if z is None: return 1.0
    if abs(z - r) <= 1e-9: return 0.0
    if z * r < 0: return 1.0
    return min(1.0, abs(z - r) / max(abs(z), abs(r)))
def row(keys, c):
    xs = [t(full[k][c]) for k in keys]
    gs = [gap(obj(full[k][c]), ref[k[0]]) if k[0] in ref else 0.0 for k in keys]
    fs = [min(full[k][c].get("first_solution_time") or 60.0, 60.0) for k in keys]
    return f"{sum(ok(full[k][c]) for k in keys)}/{len(keys)} | {sgm(xs):.2f} | {sum(gs):.3f} | {sgm(fs, 1):.2f} | {sum(obj(full[k][c]) is None for k in keys)}"
print("| 種 | 設定 | 解けた | SGM (秒) | 主ギャップの和 | 最初の解の時刻の幾何平均 (秒) | 解なし |")
print("|---|---|---|---|---|---|---|")
for sd in seeds + ["全部"]:
    keys = sorted(k for k in full if sd == "全部" or k[1] == sd)
    for c in ("base", "l"):
        print(f"| {sd} | {c} | {row(keys, c)} |")
print("\n解けた / 解けない が分かれた (問題, 種):")
for k in sorted(full):
    a, b = ok(full[k]["base"]), ok(full[k]["l"])
    if a != b: print(f"  {k[0]} 種 {k[1]}: base {'解けた' if a else '解けず'} / l {'解けた' if b else '解けず'}")
print("\n問題ごと (種を合わせた時間の SGM、比 1.3 倍以上のもの。() は種ごとの base/l の秒):")
for p in sorted({k[0] for k in full}):
    ks = sorted(k for k in full if k[0] == p)
    a, b = sgm([t(full[k]["base"]) for k in ks]), sgm([t(full[k]["l"]) for k in ks])
    if max(a, b) / max(min(a, b), 0.5) >= 1.3:
        print(f"  {p}: base {a:.1f} / l {b:.1f}  (" + ", ".join(f"{t(full[k]['base']):.0f}/{t(full[k]['l']):.0f}" for k in ks) + ")")
print("\n主ギャップが分かれた解けない問題 (種ごとの base/l の主ギャップ):")
for p in sorted({k[0] for k in full}):
    ks = sorted(k for k in full if k[0] == p)
    if all(not ok(full[k]["base"]) and not ok(full[k]["l"]) for k in ks) and p in ref:
        gb = [gap(obj(full[k]["base"]), ref[p]) for k in ks]; gl = [gap(obj(full[k]["l"]), ref[p]) for k in ks]
        if abs(sum(gb) - sum(gl)) > 1e-3:
            print(f"  {p}: base {sum(gb):.4f} / l {sum(gl):.4f}  (" + ", ".join(f"{x:.3f}/{y:.3f}" for x, y in zip(gb, gl)) + ")")
bad = [(k, c, v.get("objective")) for k, d in full.items() for c, v in d.items()
       if (ok(v) and k[0] in solu and abs(v["objective"] - solu[k[0]]) > 1e-6 * max(1, abs(solu[k[0]])) + 1e-6) or v.get("status") in ("crash", "error")]
print("\n最適値の食い違い・異常終了:", bad)

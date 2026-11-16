"""TREECLOSE の対の計測 (NAME_s*.jsonl、base 対 nb (b58g/cfgs.py)) を analysis/benchlib.py の report で集計する。
誤答 (目的値・下界の矛盾) も見る。
使い方: python summ.py [NAME]  (NAME は nb58 (既定、58 問 × 種 1〜2・60 秒) か nb15 か nb4 (診断の 15 問 × 種 1〜2・600 秒))"""
import glob, json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parents[1]))
import benchlib as bl  # noqa: E402
NAME = sys.argv[1] if len(sys.argv) > 1 else "nb58"
rows = bl.load(sorted(glob.glob(str(D / f"{NAME}_s*.jsonl"))))
R = bl.table(rows)
keys = sorted(R)
for c in ("nb",):
    bl.report(R, "base", c, keys, noise=bl.load_noise(), label=f"{NAME}: {len({k[0] for k in keys})} 問 x 種 1〜2 ({c})")
# 誤答: 最適と言った回どうしの目的値の差、下界が他の回の最適値を越えるもの (最小化)
bad = 0
for p in {k[0] for k in keys}:
    res = [R[k][c] for k in keys if k[0] == p for c in R[k]]
    opt = [x["objective"] for x in res if (x.get("status") or "").lower() == "optimal" and x.get("objective") is not None]
    if not opt:
        continue
    lo, hi = min(opt), max(opt)
    tol = 1e-6 * max(1.0, abs(lo))
    if hi - lo > tol:
        bad += 1; print("目的値の不一致", p, lo, hi)
    for x in res:
        b = x.get("bound")
        if b is not None and b > lo + tol:
            bad += 1; print("下界が最適値を越える", p, b, lo)
print("誤答", bad)
# 問題が少ない (診断の 15 問) ときは問題ごとに並べる: 種ごとの (状態, 秒, ノード)
if len({k[0] for k in keys}) <= 20:
    raw = [json.loads(l) for f in sorted(glob.glob(str(D / f"{NAME}_s*.jsonl"))) for l in open(f) if l.strip()]
    by = {}
    for r in raw:
        by.setdefault(r["problem"], {}).setdefault(r["cfg"], {})[r["seed"]] = r["result"]
    for p in sorted(by):
        cells = []
        for c in ("base", "nb"):
            v = by[p].get(c, {})
            cells.append(c + ": " + " / ".join(f"{(x.get('status') or '?')[:4]} {x.get('time', 0):.0f}s {x.get('nodes')}" for s, x in sorted(v.items())))
        print(f"{p:24} " + "  |  ".join(cells))
    # 解けた数と時間のずらし幾何平均 (ずらし 10 秒、時間切れは制限時間で頭打ち)。benchlib の report は 60 秒で頭打ちにするので 600 秒の回はここで見る
    import math
    tl = max(r.get("tl", 60) for r in raw)
    for c in ("base", "nb"):
        ts = [min(tl, x.get("time", tl)) for p in by for x in by[p].get(c, {}).values()]
        ok = sum((x.get("status") or "").lower() == "optimal" for p in by for x in by[p].get(c, {}).values())
        print(f"{c}: 解けた {ok}/{len(ts)}、ずらし幾何平均 {math.exp(sum(math.log(t + 10) for t in ts) / len(ts)) - 10:.1f} 秒 (制限 {tl} 秒)")

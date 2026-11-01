"""問題 × 設定の時間の表 (T は時間切れ、平均は T を 60 として)。根のヒューリスティクス (最初の区間) の秒の平均と、
サブ MIP の時間の和の平均も出す。使い方: python table.py 設定,... [問題,...]"""
import sys
from summ import D, parse
cfgs = sys.argv[1].split(",")
probs = sys.argv[2].split(",") if len(sys.argv) > 2 else sorted({f.stem.rsplit("_", 2)[0] for f in D.glob("*.log")})
print("| 問題 | " + " | ".join(cfgs) + " |"); print("|---" * (len(cfgs) + 1) + "|")
rows2 = []
for p in probs:
    cells, cells2 = [], []
    for c in cfgs:
        ts, hs, ss = [], [], []
        for f in sorted(D.glob(f"{p}_{c}_*.log")):
            if f.stem.rsplit("_", 2)[0] != p or f.stem.rsplit("_", 2)[1] != c: continue
            res, afts, subs = parse(f)
            if res is None: continue
            ts.append(res["time"] if res["status"] == "optimal" else None)
            if afts: hs.append(float(afts[0].split(":")[1]))
            ss.append(sum(s[5] for s in subs))
        if not ts: cells.append("-"); cells2.append("-"); continue
        a = sum(60 if t is None else t for t in ts) / len(ts)
        cells.append("・".join("T" if t is None else f"{t:.0f}" for t in ts) + f" ({a:.1f})")
        cells2.append(f"{sum(hs)/max(len(hs),1):.1f} / {sum(ss)/max(len(ss),1):.1f}")
    print(f"| {p} | " + " | ".join(cells) + " |")
    rows2.append(f"| {p} | " + " | ".join(cells2) + " |")
print("\n根のヒューリスティクスの終わりの時刻 (最初の区間) / サブ MIP の時間の和 (平均秒)\n")
print("| 問題 | " + " | ".join(cfgs) + " |"); print("|---" * (len(cfgs) + 1) + "|")
print("\n".join(rows2))

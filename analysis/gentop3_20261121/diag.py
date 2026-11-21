"""診断の集計: python diag.py。logs/diag*_*.log の `GENTOP3 n above` (集約行 1 本の向き 1 つごとの候補の数と、最良の質の 0.5 倍以上の数) を数える。
keep_best に 2 本以上の候補が来た回だけ出る (1 本以下は今までどおり素通り)。"""
import glob, re, collections
print("| 設定_問題 | 回数 | 候補の平均 | 0.5 倍以上の平均 | 1 本 | 2 本 | 3 本 | 4 本以上 |")
print("|---|---:|---:|---:|---:|---:|---:|---:|")
for f in sorted(glob.glob("logs/diag*_*.log")):
    xs = [tuple(map(int, m.groups())) for m in (re.search(r"GENTOP3 (\d+) (\d+)", l) for l in open(f)) if m]
    if not xs: continue
    n = len(xs); a = collections.Counter(min(y, 4) for _, y in xs)
    print(f"| {f[5:-4]} | {n} | {sum(x for x, _ in xs) / n:.2f} | {sum(y for _, y in xs) / n:.2f} | " + " | ".join(f"{100 * a[k] / n:.0f}%" for k in (1, 2, 3, 4)) + " |")

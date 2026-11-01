"""試作 (ENOMOTO_MIP_SUBMIP_NESTED) で作った深さ 2 のサブ MIP を種類ごとに集計する (本数・秒の和・受け入れた本数)。
使い方: python kinds.py [設定名 (既定 nclvi)] > kinds.txt"""
import sys
from collections import defaultdict
from summ import D, parse, NEST
cfg = sys.argv[1] if len(sys.argv) > 1 else "nclvi"
print("| 問題 | 種類 | 本数 | 秒の和 | 受け入れ |\n|---|---|---|---|---|")
for p in sorted({f.stem.rsplit("_", 2)[0] for f in D.glob(f"*_{cfg}_*.log")}):
    agg = defaultdict(lambda: [0, 0.0, 0])
    for f in D.glob(f"{p}_{cfg}_*.log"):
        for s in parse(f)[2].values():
            if s["depth"] == 2 and s["kind"] in NEST:
                a = agg[s["kind"]]
                a[0] += 1; a[1] += s.get("secs", 0); a[2] += s.get("acc", False)
    for k, (n, t, acc) in sorted(agg.items()):
        print(f"| {p} | {k} | {n} | {t:.1f} | {acc} |")

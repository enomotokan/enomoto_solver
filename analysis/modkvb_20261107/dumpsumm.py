"""dump/*.err.gz (ENOMOTO_MIP_MODKVB_DUMP) の集計: カットの出た解の k・行の数・LP のカットの行を含むか・変数上下限の置き換え
(連続列 vbc・一般整数列 vbi) を含むか・cover / generate_cuts のどちらがカットを出したか。使い方: python dumpsumm.py [問題 ...]"""
import collections, gzip, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
probs = sys.argv[1:] or sorted(f.name[:-7] for f in (D / "dump").glob("*.err.gz"))
for p in probs:
    sols = []; seps = []
    for l in gzip.open(D / "dump" / f"{p}.err.gz", "rt"):
        if l.startswith("MODKVB_SOL"):
            sols.append({k: int(v) for k, v in re.findall(r"(\w+) (\d+)", l)})
        elif l.startswith("SEP modkvb"):
            seps.append(l)
    if not sols:
        print(f"== {p}: 解 0 (分離 {len(seps)} 回)"); continue
    n = len(sols)
    c = lambda f: sum(1 for s in sols if f(s))
    rows = collections.Counter(min(s["rows"], 6) for s in sols)
    print(f"== {p}: カットの出た解 {n} (分離 {len(seps)} 回)")
    print(f"   k: {dict(sorted(collections.Counter(s['k'] for s in sols).items()))}  行の数 (6=6 以上): {dict(sorted(rows.items()))}")
    print(f"   LP のカットの行を含む {c(lambda s: s['cutrows'] > 0)}、連続列の変数上下限を含む {c(lambda s: s['rows_vbc'] > 0)}、一般整数列の変数上下限を含む {c(lambda s: s['rows_vbi'] > 0)}、どちらもなし {c(lambda s: s['rows_vbc'] == 0 and s['rows_vbi'] == 0)}")
    print(f"   cover が出た {c(lambda s: s['cover'] > 0)}、generate_cuts が出た {c(lambda s: s['gen'] > 0)}、集約行の長さの中央値 {sorted(s['agglen'] for s in sols)[n // 2]}")

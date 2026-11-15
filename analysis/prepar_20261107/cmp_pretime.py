"""既定と試作の前処理の段ごとの秒と、前処理の後の問題の大きさ (m, n, nnz, 自由な列) を比べる (analysis/prepar_20261107.md)。
使い方: python cmp_pretime.py 既定.jsonl 試作.jsonl"""
import json, sys
from table import stages, KEYS  # noqa: E402

def load(f):
    return {r["problem"]: r for r in map(json.loads, open(f)) if "total" in r}

A, B = load(sys.argv[1]), load(sys.argv[2])
ps = [p for p in A if p in B]
same = [p for p in ps if all(A[p][k] == B[p][k] for k in ("m", "n", "nnz", "free"))]
print(f"両方ある問題 {len(ps)}、前処理の後の大きさ (行・列・非零・自由な列) が同じ {len(same)}")
ta, tb = sum(A[p]["total"] for p in ps), sum(B[p]["total"] for p in ps)
print(f"前処理の秒の合計: 既定 {ta:.1f} → 試作 {tb:.1f} ({100 * (tb / ta - 1):+.0f}%)")
sa = {k: sum(stages(A[p])[k] for p in ps) for k in KEYS}
sb = {k: sum(stages(B[p])[k] for p in ps) for k in KEYS}
print("段ごと: " + "  ".join(f"{k} {sa[k]:.1f}→{sb[k]:.1f}" for k in KEYS if sa[k] + sb[k] > 0.5))
print("\n| 問題 | 既定 | 試作 | 比 | probe | lpfix | lpfirst | 大きさ |")
print("|---|---|---|---|---|---|---|---|")
for p in sorted(ps, key=lambda p: -A[p]["total"]):
    a, b = A[p], B[p]
    s1, s2 = stages(a), stages(b)
    size = "同じ" if p in same else f"{a['m']:.0f}x{a['free']:.0f} → {b['m']:.0f}x{b['free']:.0f}"
    print(f"| {p} | {a['total']:.1f} | {b['total']:.1f} | {b['total'] / max(a['total'], 1e-9):.2f} | {s1['probe']:.1f}→{s2['probe']:.1f} | {s1['lpfix']:.1f}→{s2['lpfix']:.1f} | {s1['lpfirst']:.1f}→{s2['lpfirst']:.1f} | {size} |")

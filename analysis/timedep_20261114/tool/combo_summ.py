"""組 C の 1 スレッドの確かめ (tool/combo.jsonl): python tool/combo_summ.py
- 決定性: 同じ設定・種 1 の 2 回 (a・b) で状態・目的値・ノード・LP の反復が同じ問題の数 (C = 組、Z = 文字なしの cand1114)。
- 遅い機械の真似 (s、同じ CPU で yes): a と同じ道の問題の数。
- 未使用の種 2・3: C と T (mplwnNebtyc、調整値は既定) の手間 (LP の反復、shift 1000) の SGM 比、解けた数、大きく変わった問題。"""
import json, math, collections
from pathlib import Path
D = Path(__file__).resolve().parent
R = collections.defaultdict(dict)
for l in open(D / "combo.jsonl"):
    r = json.loads(l); R[r["tag"], r["seed"]][r["problem"]] = r
def key(r): return (r.get("status"), r.get("objective"), r.get("nodes"), r.get("lp_iterations"))
def ok(r): return r.get("status") in ("optimal", "infeasible")
for x in "CZ":
    a = R[f"{x}a", 1]
    for y in "bs":
        b = R[f"{x}{y}", 1]; ps = [p for p in a if p in b]
        diff = [p for p in ps if key(a[p]) != key(b[p])]
        print(f"{x}a 対 {x}{y} (種 1): 同じ道 {len(ps) - len(diff)}/{len(ps)}、違う {diff}")
S = json.loads((D / "S_T.json").read_text())
w = lambda r: (r.get("lp_iterations") or 0) + 1000
for s in (2, 3):
    c, t = R["Ca", s], R["Ta", s]; ps = [p for p in S if p in c and p in t]
    if not ps: continue
    lw = [math.log(w(c[p]) / w(t[p])) for p in ps]
    up = [(p, round(math.exp(v), 2)) for p, v in zip(ps, lw) if v > math.log(1.5)]
    dn = [(p, round(math.exp(v), 2)) for p, v in zip(ps, lw) if v < math.log(2 / 3)]
    lost = [p for p in ps if ok(t[p]) and not ok(c[p])]; won = [p for p in ps if ok(c[p]) and not ok(t[p])]
    print(f"種 {s}: C 対 T {len(ps)} 問、手間 {math.exp(sum(lw) / len(lw)) - 1:+.1%}、解けた T {sum(ok(t[p]) for p in ps)} / C {sum(ok(c[p]) for p in ps)} (落とす {lost}、得る {won})")
    print("  手間 ×1.5 以上:", up, "\n  手間 ×2/3 以下:", dn)
c, z = R["Ca", 1], R["Za", 1]; ps = [p for p in c if p in z]
if ps:
    lw = [math.log(w(c[p]) / w(z[p])) for p in ps]
    print(f"種 1: C 対 文字なし {len(ps)} 問、手間 {math.exp(sum(lw) / len(lw)) - 1:+.1%}、解けた Z {sum(ok(z[p]) for p in ps)} / C {sum(ok(c[p]) for p in ps)}")

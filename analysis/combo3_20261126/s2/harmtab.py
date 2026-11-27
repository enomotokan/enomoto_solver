"""c3 の遅れ (1.25 倍超) の問題の表 (README 8 節): python analysis/combo3_20261126/s2/harmtab.py → 標準出力。
今回 (cd10d12f7) の def・c3 の時間・ノード・根の下界・最初の解の時刻と、前回 (combo2 の第 2 段、c8e468e25) の def・c2c の時間 (c2c / def の比) を並べる。"""
import glob, json, math
from pathlib import Path
D = Path(__file__).resolve().parent
OLD = D.parents[1] / "combo2_20261125" / "s2"
TL, SH = 600.0, 10.0
def load(pat):
    R = {}
    for g in sorted(glob.glob(str(pat))):
        for l in open(g):
            if l.strip(): r = json.loads(l); R[(r["problem"], r["cfg"])] = r["result"]
    return R
N, O = load(D / "res_s*.jsonl"), load(OLD / "res_s*.jsonl")
def st(x): return str(x.get("status") or "").lower()
def t(x): return min(x.get("time") or TL, TL) if st(x) in ("optimal", "infeasible") else TL
def ratio(a, b): return (t(a) + SH) / (t(b) + SH)
for th in ("t1", "t4"):
    rows = []
    for (p, c), a in N.items():
        if c != f"c3_{th}" or (p, f"def_{th}") not in N: continue
        b = N[(p, f"def_{th}")]
        if st(a) in ("crash", "killed") or st(b) in ("crash", "killed"): continue
        r = ratio(a, b)
        if r > 1.25: rows.append((r, p, a, b))
    print(f"\n== {th}: c3 / def が 1.25 倍超の {len(rows)} 問\n")
    print("| 問題 | 比 | def 秒 / ノード | c3 秒 / ノード | 根の下界 def / c3 | 最初の解 (秒) def / c3 | 前回 c2c / def |")
    print("|---|---:|---|---|---|---|---|")
    for r, p, a, b in sorted(rows, key=lambda x: -x[0]):
        oa, ob = O.get((p, f"c2c_{th}")), O.get((p, f"def_{th}"))
        old = f"{ratio(oa, ob):.2f} ({t(ob):.0f} → {t(oa):.0f})" if oa and ob else "—"
        f = lambda x: f"{t(x):.0f}{'' if st(x) in ('optimal','infeasible') else '*'} / {x.get('nodes')}"
        rb = lambda x: f"{x.get('root_bound'):.6g}" if isinstance(x.get('root_bound'), (int, float)) else "—"
        fs = lambda x: f"{x.get('first_solution_time'):.1f}" if isinstance(x.get('first_solution_time'), (int, float)) else "—"
        print(f"| {p} | {r:.2f} | {f(b)} | {f(a)} | {rb(b)} / {rb(a)} | {fs(b)} / {fs(a)} | {old} |")
print("\n(* は時間切れ)")

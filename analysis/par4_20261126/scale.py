"""combo2 第 2 段 (analysis/combo2_20261125/s2、240 問・600 秒・種 1) の結果から、4 スレッドが 1 スレッドに比べて伸びていない問題と、
c2c の効き目が 1 スレッドにはあって 4 スレッドで消える問題を数で出す (README 1 節)。
    python analysis/par4_20261126/scale.py > scale.txt
- 時間はずらし 10 秒 (解けない回は 600 秒)。伸び = (1T + 10) / (4T + 10) (4 倍が理想、1 未満は 4 スレッドのほうが遅い)。
- c2c の効き目 = (c2c + 10) / (def + 10)。1T と 4T の効き目の比 e4 / e1 が大きいほど、4 スレッドで効き目が消える。"""
import glob, json, math
from pathlib import Path
D = Path(__file__).resolve().parent
S2 = D.parent / "combo2_20261125" / "s2"
R = {}
for f in glob.glob(str(S2 / "res_s*.jsonl")):
    for l in open(f):
        if l.strip():
            r = json.loads(l); k = (r["problem"], r["cfg"])
            if k not in R or str(r["result"].get("status")).lower() not in ("crash", "fetch_error"): R[k] = r
TL, SH = 600.0, 10.0
def st(r): return str(r["result"].get("status") or "").lower()
def t(r): return min(r["result"].get("time") or TL, TL) if st(r) in ("optimal", "infeasible") else TL
def nodes(r): return r["result"].get("nodes")
PJ = json.loads((D.parent / "combo_20261124" / "problems.json").read_text()); M = set(PJ["M"])
probs = sorted({p for p, _ in R})
rows = []
for p in probs:
    try: a1, a4, c1, c4 = (R[(p, c)] for c in ("def_t1", "def_t4", "c2c_t1", "c2c_t4"))
    except KeyError: continue
    if any(st(x) in ("crash", "killed", "") for x in (a1, a4, c1, c4)): continue
    if max(t(a1), t(a4), t(c1), t(c4)) < 1.0: continue
    sp_d = (t(a1) + SH) / (t(a4) + SH); sp_c = (t(c1) + SH) / (t(c4) + SH)
    e1 = (t(c1) + SH) / (t(a1) + SH); e4 = (t(c4) + SH) / (t(a4) + SH)
    rows.append(dict(p=p, M=p in M, d1=t(a1), d4=t(a4), c1=t(c1), c4=t(c4), sp_d=sp_d, sp_c=sp_c, e1=e1, e4=e4, n1=nodes(a1), n4=nodes(a4), cn1=nodes(c1), cn4=nodes(c4)))
both_tl = [r for r in rows if max(r["d1"], r["d4"], r["c1"], r["c4"]) >= TL and min(r["d1"], r["d4"], r["c1"], r["c4"]) >= TL]
info = [r for r in rows if r not in both_tl]
g = lambda xs: math.exp(sum(math.log(x) for x in xs) / len(xs))
print(f"組 {len(rows)} 問 (4 つの回とも有効)、うち 4 つとも時間切れ {len(both_tl)} 問、残り {len(info)} 問で数える\n")
print(f"伸び (1T / 4T、ずらし 10 秒) の幾何平均: 既定 {g([r['sp_d'] for r in info]):.3f}、c2c {g([r['sp_c'] for r in info]):.3f}")
print(f"c2c の効き目 (c2c / 既定) の幾何平均: 1T {g([r['e1'] for r in info]):.3f}、4T {g([r['e4'] for r in info]):.3f}\n")
for lab, key in (("既定", "sp_d"), ("c2c", "sp_c")):
    xs = [r[key] for r in info]
    print(f"{lab} の伸びの分布: < 1 (4T のほうが遅い) {sum(x < 1 for x in xs)}、1〜1.5 {sum(1 <= x < 1.5 for x in xs)}、1.5〜2.5 {sum(1.5 <= x < 2.5 for x in xs)}、2.5 以上 {sum(x >= 2.5 for x in xs)} (計 {len(xs)})")
print("\n== 既定で 4 スレッドが伸びていない問題 (伸び < 1.2、1T か 4T が 20 秒以上、伸びの小さい順)\n")
print("| 問題 | M | 既定 1T | 既定 4T | 伸び | ノード 1T → 4T | c2c 1T | c2c 4T | c2c 伸び |")
print("|---|---|---:|---:|---:|---|---:|---:|---:|")
for r in sorted([r for r in info if r["sp_d"] < 1.2 and max(r["d1"], r["d4"]) >= 20], key=lambda r: r["sp_d"]):
    print(f"| {r['p']} | {'M' if r['M'] else ''} | {r['d1']:.1f} | {r['d4']:.1f} | {r['sp_d']:.2f} | {r['n1']} → {r['n4']} | {r['c1']:.1f} | {r['c4']:.1f} | {r['sp_c']:.2f} |")
print("\n== c2c の効き目が 1T にはあって 4T で消える (か害になる) 問題 (e1 < 0.8 で e4 > 0.95、または e4 / e1 > 1.5、e4 / e1 の大きい順)\n")
print("| 問題 | M | 既定 1T | c2c 1T | 効き目 1T | 既定 4T | c2c 4T | 効き目 4T | 4T / 1T | ノード c2c 1T / 4T |")
print("|---|---|---:|---:|---:|---:|---:|---:|---:|---|")
for r in sorted([r for r in info if (r["e1"] < 0.8 and r["e4"] > 0.95) or r["e4"] / r["e1"] > 1.5], key=lambda r: -r["e4"] / r["e1"]):
    print(f"| {r['p']} | {'M' if r['M'] else ''} | {r['d1']:.1f} | {r['c1']:.1f} | {r['e1']:.2f} | {r['d4']:.1f} | {r['c4']:.1f} | {r['e4']:.2f} | {r['e4']/r['e1']:.2f} | {r['cn1']} / {r['cn4']} |")

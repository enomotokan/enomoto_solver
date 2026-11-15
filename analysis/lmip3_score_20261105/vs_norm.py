"""組ごとの 3 秒の値の正規化した平均: 問題ごとに全部の組の最良を 0・最悪を 1 (解なしも 1) にして平均。python -I vs_norm.py base ..."""
import re, glob, os, sys
D = os.path.dirname(os.path.abspath(__file__))
names = sys.argv[1:]
pat = re.compile(r"lm first (\S+) t (\S+) best (\S+) least_viol (\d+) steps (\d+)")
V = {}
for nm in names:
    for f in glob.glob(f"{D}/logs/vs_{nm}/*.log"):
        p = os.path.basename(f)[:-4]
        l = next((l for l in open(f, errors="replace") if l.startswith("FJVSLM")), None)
        g = pat.search(l).groups() if l else None
        V.setdefault(p, {})[nm] = None if g is None or g[2] == "none" else float(g[2])
tot = {n: 0.0 for n in names}; cnt = 0; win = {n: 0 for n in names}
for p, d in V.items():
    vals = [v for v in d.values() if v is not None]
    if not vals or len(d) < len(names): continue
    lo, hi = min(vals), max(vals)
    if hi - lo <= 1e-6 * max(1, abs(lo)) and all(d[n] is not None for n in names): continue  # 全部同じ
    cnt += 1
    for n in names:
        v = d[n]
        tot[n] += 1.0 if v is None else ((v - lo) / (hi - lo) if hi > lo else 0.0)
        if v is not None and abs(v - lo) <= 1e-6 * max(1, abs(lo)): win[n] += 1
print(f"組で差のある {cnt} 問の正規化した平均 (0 = その問題の最良、1 = 最悪か解なし) と、最良を取った問題の数:")
for n in names: print(f"- {n}: {tot[n]/cnt:.3f}、最良 {win[n]} 問")

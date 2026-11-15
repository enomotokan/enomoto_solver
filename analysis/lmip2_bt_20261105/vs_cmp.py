"""run_vs.sh の FJVSLM の行を設定ごとに並べる: python -I vs_cmp.py base b l w blw ...
Local-MIP の最初の解の時刻と 3 秒の最良の値を、最初の設定 (base) と比べる。"""
import re, sys, glob, os, math
D = os.path.dirname(os.path.abspath(__file__))
cfgs = sys.argv[1:] or ["base", "b", "l", "w", "blw"]
pat = re.compile(r"lm first (\S+) t (\S+) best (\S+) least_viol (\d+) steps (\d+)")
def load(p, c):
    f = f"{D}/logs/{p}_{c}.log"
    if not os.path.exists(f): return None
    for l in open(f, errors="replace"):
        if l.startswith("FJVSLM"):
            m = pat.search(l)
            if m:
                lf, lt, lb, lv, st = m.groups()
                return (None if lt == "-" else float(lt), None if lb == "none" else float(lb), int(lv), int(st))
    return None
probs = sorted({os.path.basename(f).rsplit("_", 1)[0] for f in glob.glob(f"{D}/logs/*_base.log")})
def better(a, b):  # 値 a が b より良い (最小化) なら 1、悪いなら -1
    if a is None and b is None: return 0
    if a is None: return -1
    if b is None: return 1
    tol = 1e-6 * max(1.0, abs(b))
    return 1 if a < b - tol else (-1 if a > b + tol else 0)
print("| 問題 | " + " | ".join(f"{c} 最初 (秒) / 3 秒の最良 / 最少違反" for c in cfgs) + " |")
print("|---|" + "---|" * len(cfgs))
summ = {c: [0, 0, 0, 0, []] for c in cfgs}  # 良い, 悪い, 同じ, 解の数, 最初の時刻
for p in probs:
    r = {c: load(p, c) for c in cfgs}
    cells = []
    for c in cfgs:
        x = r[c]
        if x is None: cells.append("(なし)"); continue
        t, b, lv, st = x
        mark = ""
        if c != cfgs[0] and r[cfgs[0]] is not None:
            k = better(b, r[cfgs[0]][1]); mark = {1: " ▲", -1: " ▼", 0: ""}[k]
            summ[c][{1: 0, -1: 1, 0: 2}[k]] += 1
        if b is not None: summ[c][3] += 1
        summ[c][4].append(t)
        cells.append(("解なし" if b is None else f"{t:.3f} / {b:.6g}") + f" / {lv}{mark}")
    print(f"| {p} | " + " | ".join(cells) + " |")
print()
print("| 設定 | 解あり | base より良い | 悪い | 同じ | 最初の解の時刻の幾何平均 (shift 0.01、両方解ありの問題) |")
print("|---|---|---|---|---|---|")
base_t = summ[cfgs[0]][4]
for c in cfgs:
    g, bd, eq, ns, ts = summ[c]
    pairs = [(a, b) for a, b in zip(ts, base_t) if a is not None and b is not None]
    gm = math.exp(sum(math.log(a + 0.01) for a, _ in pairs) / max(1, len(pairs))) - 0.01
    gb = math.exp(sum(math.log(b + 0.01) for _, b in pairs) / max(1, len(pairs))) - 0.01
    print(f"| {c} | {ns} | {g} | {bd} | {eq} | {gm:.4f} (base {gb:.4f}, {len(pairs)} 問) |")
# 問題ごとに、並べた設定の中の最良を 0・最悪を 1 にした値 (解なしは 1) の平均 (どの設定も同じ値・どれも解なしの問題は除く)
acc = {c: [] for c in cfgs}
for p in probs:
    r = {c: load(p, c) for c in cfgs}
    if any(r[c] is None for c in cfgs): continue
    vals = [r[c][1] for c in cfgs if r[c][1] is not None]
    if not vals: continue
    lo, hi = min(vals), max(vals)
    if hi - lo <= 1e-6 * max(1, abs(lo)) and len(vals) == len(cfgs): continue
    for c in cfgs:
        v = r[c][1]
        acc[c].append(1.0 if v is None else ((v - lo) / (hi - lo) if hi > lo else 0.0))
print("\n| 設定 | 正規化した 3 秒の値の平均 (0 = 並べた設定の最良、1 = 最悪・解なし) |")
print("|---|---|")
for c in cfgs:
    print(f"| {c} | {sum(acc[c]) / max(1, len(acc[c])):.3f} ({len(acc[c])} 問) |")

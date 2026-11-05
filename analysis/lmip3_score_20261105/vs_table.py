"""組ごとの Local-MIP (3 秒) の比べ: python -I vs_table.py base s a t sat"""
import re, glob, os, sys, math
D = os.path.dirname(os.path.abspath(__file__))
names = sys.argv[1:]
pat = re.compile(r"lm first (\S+) t (\S+) best (\S+) least_viol (\d+) steps (\d+)")
data = {}
for nm in names:
    for f in glob.glob(f"{D}/logs/vs_{nm}/*.log"):
        p = os.path.basename(f)[:-4]
        l = next((l for l in open(f, errors="replace") if l.startswith("FJVSLM")), None)
        g = pat.search(l).groups() if l else None
        data.setdefault(p, {})[nm] = g
probs = sorted(data)
num = lambda s: None if s == "none" else float(s)
print("| 問題 | " + " | ".join(f"{n} 最初 (秒) / 3 秒の値 / 最少違反" for n in names) + " |")
print("|---|" + "---|" * len(names))
base = names[0]
summ = {n: {"better": 0, "worse": 0, "same": 0, "found": 0, "ft": []} for n in names}
for p in probs:
    cells = []
    for n in names:
        g = data[p].get(n)
        if g is None:
            cells.append("(なし)"); continue
        lf, lt, lb, lv, st = g
        if lb != "none":
            summ[n]["found"] += 1
        cells.append(f"{'-' if lt == '-' else lt} / {'解なし' if lb == 'none' else f'{float(lb):.6g}'} / {lv}")
        gb = data[p].get(base)
        if n != base and gb is not None:
            a, b = num(gb[2]), num(lb)
            if a is None and b is None: s = "same"
            elif a is None: s = "better"
            elif b is None: s = "worse"
            elif abs(a - b) <= 1e-6 * max(1, abs(a)): s = "same"
            else: s = "better" if b < a else "worse"
            summ[n][s] += 1
    print(f"| {p} | " + " | ".join(cells) + " |")
# 最初の解の時刻の幾何平均 (全部の組で解が出た問題だけ、+0.01 秒)
common = [p for p in probs if all(data[p].get(n) and data[p][n][1] != "-" for n in names)]
print(f"\n全部の組で解が出た {len(common)} 問での、最初の解の時刻の幾何平均 (+0.01 秒):")
for n in names:
    gm = math.exp(sum(math.log(float(data[p][n][1]) + 0.01) for p in common) / max(1, len(common))) - 0.01
    s = summ[n]
    print(f"- {n}: 解あり {s['found']} 問、幾何平均 {gm:.3f} 秒" + ("" if n == base else f"、3 秒の値が {base} より良い {s['better']}・悪い {s['worse']}・同じ {s['same']}"))

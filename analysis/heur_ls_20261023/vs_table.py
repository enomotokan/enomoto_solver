"""batch5 (FJVSLM) の表: python -I vs_table.py"""
import re, glob, os
D = os.path.dirname(os.path.abspath(__file__))
rows = []
for f in sorted(glob.glob(f"{D}/logs/*_vs_1.log")):
    p = os.path.basename(f)[:-len("_vs_1.log")]
    l = next((l for l in open(f, errors="replace") if l.startswith("FJVSLM")), None)
    if not l: rows.append((p, None)); continue
    m = re.search(r"fj1 found (\S+) t (\S+) \| fj \(\S+\) best (\S+) t (\S+) \| lm first (\S+) t (\S+) best (\S+) least_viol (\d+)", l)
    rows.append((p, m.groups()))
def num(s): return None if s == "none" else float(s)
cnt = {"lm": 0, "fj": 0, "tie": 0, "none": 0}
print("| 問題 | FJ 1 回目 (秒) | FJ 3 秒の最良 | LM 最初 (秒) | LM 3 秒の最良 | LM 最少違反 | 良い方 |")
print("|---|---|---|---|---|---|---|")
for p, g in rows:
    if g is None: print(f"| {p} | (行なし) | | | | | |"); continue
    f1, t1, fb, t2, lf, lt, lb, lv = g
    fbv, lbv = num(fb), num(lb)
    if fbv is None and lbv is None: w = "どちらも解なし"; cnt["none"] += 1
    elif fbv is None: w = "LM (FJ 解なし)"; cnt["lm"] += 1
    elif lbv is None: w = "FJ (LM 解なし)"; cnt["fj"] += 1
    elif abs(fbv - lbv) <= 1e-6 * max(1, abs(fbv)): w = "同じ"; cnt["tie"] += 1
    elif lbv < fbv: w = "LM"; cnt["lm"] += 1
    else: w = "FJ"; cnt["fj"] += 1
    fmt = lambda s: "解なし" if s == "none" else f"{float(s):.6g}"
    print(f"| {p} | {fmt(f1)} ({float(t1):.2f}) | {fmt(fb)} | {fmt(lf)} ({lt}) | {fmt(lb)} | {lv} | {w} |")
print(f"\nLM が良い {cnt['lm']}、FJ が良い {cnt['fj']}、同じ {cnt['tie']}、どちらも解なし {cnt['none']}")

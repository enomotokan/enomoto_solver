"""vs55.sh の表: python -I vs_table.py。既定 (def) と試作 (c1) の Local-MIP の最初の解の時刻・3 秒の最良・手の数を並べる"""
import re, glob, os
D = os.path.dirname(os.path.abspath(__file__))
PAT = re.compile(r"lm first (\S+) t (\S+) best (\S+) least_viol (\d+) steps (\d+)")
def get(p, c):
    f = f"{D}/logs/{p}_{c}.log"
    if not os.path.exists(f): return None
    txt = open(f, errors="replace").read()
    l = next((l for l in txt.splitlines() if l.startswith("FJVSLM")), None)
    if not l: return None
    m = PAT.search(l)
    fb = re.search(r"fj \(\S+\) best (\S+)", l).group(1)
    cs = re.findall(r"cand1: (\d+) steps, collected (\d+) evaluated (\d+) binary duplicates skipped (\d+)", txt)
    return m.groups() + (fb, cs[0] if cs else None)
num = lambda s: None if s == "none" else float(s)
probs = sorted({os.path.basename(f).rsplit("_", 1)[0] for f in glob.glob(f"{D}/logs/*_def.log")})
cnt = {"c1": 0, "def": 0, "tie": 0, "none": 0}
ft = {"c1": 0, "def": 0, "tie": 0}
rat = []
print("| 問題 | FJ 3 秒 | 既定 最初 (秒) | 既定 3 秒 | 既定 最少違反 | 既定 手数 | 試作 最初 (秒) | 試作 3 秒 | 試作 最少違反 | 試作 手数 | 試作 評価/手 | 0-1 重複 | 3 秒で良い方 |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
for p in probs:
    a, b = get(p, "def"), get(p, "c1")
    if a is None or b is None:
        print(f"| {p} | (行なし) |" + " |" * 11); continue
    av, bv = num(a[2]), num(b[2])
    if av is None and bv is None: w = "どちらも解なし"; cnt["none"] += 1
    elif av is None: w = "**試作** (既定 解なし)"; cnt["c1"] += 1
    elif bv is None: w = "**既定** (試作 解なし)"; cnt["def"] += 1
    elif abs(av - bv) <= 1e-6 * max(1, abs(av)): w = "同じ"; cnt["tie"] += 1
    elif bv < av: w = "試作"; cnt["c1"] += 1
    else: w = "既定"; cnt["def"] += 1
    if a[1] != "-" and b[1] != "-":
        ta, tb = float(a[1]), float(b[1])
        if abs(ta - tb) <= 0.01: ft["tie"] += 1
        elif tb < ta: ft["c1"] += 1
        else: ft["def"] += 1
    if int(a[4]) > 0 and int(b[4]) > 0: rat.append(int(b[4]) / int(a[4]))
    f = lambda s: "解なし" if s == "none" else f"{float(s):.6g}"
    ev = f"{int(b[6][2]) / max(1, int(b[6][0])):.0f}" if b[6] else "-"
    dup = b[6][3] if b[6] else "-"
    print(f"| {p} | {f(a[5])} | {f(a[0])} ({a[1]}) | {f(a[2])} | {a[3]} | {a[4]} | {f(b[0])} ({b[1]}) | {f(b[2])} | {b[3]} | {b[4]} | {ev} | {dup} | {w} |")
import math
print(f"\n3 秒の最良: 試作が良い {cnt['c1']}、既定が良い {cnt['def']}、同じ {cnt['tie']}、どちらも解なし {cnt['none']}")
print(f"最初の解の時刻 (両方解あり、差 0.01 秒以内は同じ): 試作が早い {ft['c1']}、既定が早い {ft['def']}、同じ {ft['tie']}")
if rat: print(f"手数の比 (試作 / 既定) の幾何平均 {math.exp(sum(map(math.log, rat)) / len(rat)):.3f} (最小 {min(rat):.3f}、最大 {max(rat):.3f}、{len(rat)} 問)")

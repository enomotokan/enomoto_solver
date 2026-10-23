# bench.sh のログの表: python bsum.py <名前> [<名前>...]  (根の下界 = 最初の "cut loop done" の obj の後の値、1 ラウンドの時間 = 最初の根のループの (sep+LP)/ラウンド)
import re, glob, sys, os
D = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'logs')
tags = ['def'] + sys.argv[1:]
PS = "roll3000 mzzv42z 30n20b8 neos-911970 physiciansched6-2 misc07 mik-250-20-75-4 qnet1 10teams neos5 h80x6320d beasleyC3 mc11".split()
def one(p, t, r):
    f = f"{D}/b_{p}_{t}_{r}"
    if not os.path.exists(f + '.out'):
        return None
    out = open(f + '.out').read(); err = open(f + '.err').read()
    m = re.search(r"RESULT t=([\d.]+) status=(\S+) obj=(\S+) bound=(\S+)", out)
    if not m:
        return None
    t_, st, ob, bd = m.groups()
    rb = re.search(r"cut loop done: .*-> (\S+)", err)
    ft = re.search(r"FACETIME root restarts 0 rounds (\d+): sep ([\d.]+)s .*LP ([\d.]+)s", err)
    rnd = f"{1e3*(float(ft.group(2))+float(ft.group(3)))/max(1,int(ft.group(1))):.0f}ms/{ft.group(1)}" if ft else "-"
    solved = st == 'optimal'
    return (float(t_) if solved else None, st, rb.group(1) if rb else '-', rnd, bd)
print("| 問題 | " + " | ".join(tags) + " |")
print("|---|" + "---|" * len(tags))
for p in PS:
    cells = []
    for t in tags:
        rs = [one(p, t, r) for r in (1, 2)]
        cells.append("; ".join("-" if x is None else (f"{x[0]:.1f}s" if x[0] is not None else f"TL({float(x[4]):.6g})") + f" 根 {float(x[2]):.6g} ({x[3]})" if x[2] != '-' else ("-" if x is None else f"{x[1]}") for x in rs))
    print(f"| {p} | " + " | ".join(cells) + " |")

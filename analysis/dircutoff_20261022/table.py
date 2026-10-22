# 60 秒の結果の表: 問題ごとに設定 × 回の (時間 or TL(下界)) と根の最初のループの下界・暫定解の有無
import re, sys, glob, os
P = "roll3000 mzzv42z neos-911970 physiciansched6-2 beasleyC3 mc11 misc07 mik-250-20-75-4 qnet1 10teams neos5 h80x6320d p0201".split()
C = sys.argv[1:] or ["def", "dcd", "ndcd"]
D = os.path.dirname(os.path.abspath(__file__)) + "/logs/"
def one(p, c, i):
    e = D + f"b_{p}_{c}_{i}.err"; o = e[:-4] + ".out"
    if not os.path.exists(o): return "-"
    r = open(o).read(); m = re.search(r"t=(\S+) status=(\S+) obj=(\S+) bound=(\S+)", r)
    if not m: return "?"
    t, st, b = float(m.group(1)), m.group(2), float(m.group(4))
    root = None; inc = ""
    for l in open(e, errors="replace"):
        if l.startswith("CUTSEL_DCD: round") and inc == "": inc = "i" if "incumbent" in l else "n"
        mm = re.match(r"MIP: cut loop done: .* -> (\S+)", l)
        if mm and root is None: root = float(mm.group(1))
    s = f"{t:.1f}" if st == "optimal" else f"TL({b:.6g})"
    return f"{s} [{root:.6g}{inc}]" if root is not None else s
print("| 問題 | " + " | ".join(C) + " |"); print("|---" * (len(C) + 1) + "|")
for p in P:
    print(f"| {p} | " + " | ".join("、".join(one(p, c, i) for i in (1, 2)) for c in C) + " |")

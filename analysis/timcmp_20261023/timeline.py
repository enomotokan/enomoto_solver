"""timeline.py <ログ> ...: 10 秒ごとの (ノード数, 下界, 暫定解) を表にする (ENOMOTO の進み行・HiGHS の表の行)。"""
import re, sys
TS = [5, 10, 20, 30, 40, 50, 60, 70, 80]
def rows(path):
    out = []
    for l in open(path):
        m = re.match(r"\[\s*([\d.]+)\]\s+([\d.]+)s\s+(\d+)\s+(\d+)\s+(\S+)\s+(\S+)\s+\S+%", l)
        if m:  # ENOMOTO
            out.append((float(m[1]), int(m[3]), float(m[5]), float(m[6]) if m[6] != "inf" else None)); continue
        m = re.match(r"\[\s*([\d.]+)\]\s+[A-Za-z]?\s+(\d+)\s+\d+\s+\d+\s+[\d.]+%\s+(\S+)\s+(\S+)\s+", l)
        if m:  # HiGHS
            try: out.append((float(m[1]), int(m[2]), float(m[3]), float(m[4]) if m[4] != "inf" else None))
            except ValueError: pass
    return out
print("| ログ | " + " | ".join(f"{t}s" for t in TS) + " |")
print("|---|" + "---|" * len(TS))
for p in sys.argv[1:]:
    r = rows(p); cells = []
    for t in TS:
        prev = [x for x in r if x[0] <= t]
        if not prev or (r and r[-1][0] < t - 3 and t > r[-1][0]): cells.append("-" if not prev or r[-1][0] < t-3 else ""); continue
        x = prev[-1]; cells.append(f"{x[1]} / {x[2]/1e3:.0f}k / {x[3]/1e3:.0f}k" if x[3] else f"{x[1]} / {x[2]/1e3:.0f}k / -")
    print(f"| {p.split('/')[-1][8:-4]} | " + " | ".join(cells) + " |")

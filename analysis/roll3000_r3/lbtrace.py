# 通しの時刻ごとの下界・上界・ノード数: python lbtrace.py <log> [t1 t2 ...]
import re, sys
fn = sys.argv[1]; ts = [float(t) for t in sys.argv[2:]] or [20, 30, 40, 50, 60, 80, 100]
off = 0.0; pts = []
for ln in open(fn, errors="replace"):
    m = re.search(r"Restart \d+: .*\(([\d.]+)s\)", ln)
    if m: off += float(m.group(1)); continue
    m = re.match(r"\s+([\d.]+)s\s+(\d+)\s+(\d+)\s+(\S+)\s+(\S+)", ln)
    if m: pts.append((off + float(m.group(1)), int(m.group(2)), float(m.group(4)), float(m.group(5))))
row = []
for t in ts:
    c = [p for p in pts if p[0] <= t]
    row.append(f"{t:.0f}s: n{c[-1][1]} lb{c[-1][2]:.0f} ub{c[-1][3]:.0f}" if c else f"{t:.0f}s: -")
print(fn.split("/")[-1], " | ".join(row))

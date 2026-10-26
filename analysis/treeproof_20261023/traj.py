"""木の進み (通しの時刻・ノード・下界・上界) を 5 秒ごとに出す: python3 traj.py <log>...
再スタートで 0 に戻る時刻は "Restart k: ... (T s)" の T を足して通しの時刻にする。"""
import re, sys
for f in sys.argv[1:]:
    off = 0.0; rows = []
    for l in open(f):
        m = re.match(r"Restart \d+: .*\((\d+\.\d+)s\)", l)
        if m: off += float(m.group(1)); continue
        m = re.match(r"\s+(\d+\.\d)s\s+(\d+)\s+(\d+)\s+(\S+)\s+(\S+)\s+(\S+)%", l)
        if m: rows.append((off + float(m.group(1)), int(m.group(2)), int(m.group(3)), float(m.group(4)), m.group(5)))
    out = []; nxt = 0
    for r in rows:
        if r[0] >= nxt: out.append(r); nxt = (r[0] // 5 + 1) * 5
    if rows and out[-1] != rows[-1]: out.append(rows[-1])
    print(f.split('/')[-1], " | ".join(f"{t:.0f}s n{n} lb{lb:.6g} ub{ub}" for t, n, o, lb, ub in out))

"""ENOMOTO の詳細ログ (gz) から、主の進み具合の行 (`MIP: <秒>s nodes .. open .. lb .. ub ..`) を拾い、だいたい every 秒ごとに出す。
使い方: python traj.py logs/X.err.gz [every]"""
import gzip, re, sys
every = float(sys.argv[2]) if len(sys.argv) > 2 else 30.0
pat = re.compile(r"^MIP:\s+([0-9.]+)s nodes\s+(\d+) open\s+(\d+) lb (\S+) ub (\S+)")
nxt = 0.0
last = None
off = 0.0  # 再スタートで時刻が始めから数え直しにならないよう、時刻が戻ったら直前の値を足す
prev = 0.0
for line in gzip.open(sys.argv[1], "rt", errors="replace"):
    m = pat.match(line)
    if not m:
        continue
    t = float(m.group(1))
    last = (t, m.group(2), m.group(3), m.group(4), m.group(5))
    if t >= nxt:
        print(f"{t:8.1f}s nodes {m.group(2):>8} open {m.group(3):>7} lb {float(m.group(4)):.10g} ub {float(m.group(5)):.10g}")
        nxt = t + every
if last:
    print(f"{last[0]:8.1f}s nodes {last[1]:>8} open {last[2]:>7} lb {float(last[3]):.10g} ub {float(last[4]):.10g} (最後)")

#!/usr/bin/env python3
"""ENOMOTO (`ENOMOTO_MIP_TREE4_TRACE` の `TREE4:` の行 (全スレッドの下界・ノード)、なければ `MIP:  t s nodes ..` の行) と HiGHS (表の行) のログから、60 秒ごとの 下界・暫定解・ノード・開いたノード を出す。
使い方: traj.py ログ [刻み秒]"""
import re, sys
f = sys.argv[1]; step = float(sys.argv[2]) if len(sys.argv) > 2 else 60
pts = []; tree4 = False
for l in open(f, errors='replace'):
    m = re.match(r'TREE4: t ([\d.]+) tid \d+ glb (\S+) ub (\S+) lim \S+ gnodes (\d+) gopen (\d+)', l)
    if m:
        tree4 = True
        pts.append((float(m[1]), m[2], m[3], int(m[4]), int(m[5]))); continue
    if tree4: continue
    m = re.match(r'MIP:\s+([\d.]+)s nodes\s+(\d+) open\s+(\d+) lb (\S+) ub (\S+)', l)
    if m:
        pts.append((float(m[1]), m[4], m[5], int(m[2]), int(m[3]))); continue
    m = re.match(r'\s*[A-Za-z]?\s+(\d+)\s+(\d+)\s+(\d+)\s+([\d.]+)%\s+(\S+)\s+(\S+)\s+\S+\s+.*\s([\d.]+)s\s*$', l)
    if m:
        pts.append((float(m[7]), m[5], m[6], int(m[1]), int(m[2])))
t = step; last = None
for p in pts:
    while p[0] >= t:
        if last: print(f"{t:5.0f}s lb {last[1]:>14} ub {last[2]:>14} nodes {last[3]:>7} open {last[4]:>6}")
        t += step
    last = p
if last: print(f"end {last[0]:.0f}s lb {last[1]:>14} ub {last[2]:>14} nodes {last[3]:>7} open {last[4]:>6}")

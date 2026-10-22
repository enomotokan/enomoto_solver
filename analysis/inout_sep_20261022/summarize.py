"""ログの一覧: 根のループ (最初の求解) の ラウンド数・入れたカット数・根の下界・時間、in-out の集計、背景のカット、結果。
使い方: python3 summarize.py logs/*.log"""
import re, sys
for f in sorted(sys.argv[1:]):
    t = open(f, errors="replace").read()
    name = f.split("/")[-1][:-4]
    rounds = re.findall(r"MIP: cut round (\d+): (\d+) cuts .*obj ([-0-9.e+]+) \(([0-9.]+)s\)", t)
    rc = re.search(r"Root cuts: objective ([-0-9.e+]+) -> ([-0-9.e+]+), (\d+) cuts in the LP \(([0-9.]+)s\)", t)
    io = re.search(r"MIP: in-out separation: (\d+) rounds, (\d+) candidates, (\d+) cuts added \((\d+) also cut x_lp\), center moved (\d+) times, ([0-9.]+)s", t)
    bg = re.search(r"background cuts: (\d+) rounds, .*obj ([-0-9.e+]+) -> ([-0-9.e+]+)", t)
    res = re.search(r"RESULT (\S+) (\S+) (\S+) (\d+) ([0-9.]+) (\S+)", t)
    # 最初の根のループのラウンド (ラウンド 0 が 2 回目に出るまで)
    first = []
    for r in rounds:
        if r[0] == "0" and first:
            break
        first.append(r)
    added = sum(int(r[1]) for r in first)
    s = f"{name:32s} rounds {len(first):3d} added {added:5d}"
    if rc:
        s += f" root {float(rc.group(1)):.6g}->{float(rc.group(2)):.6g} ({rc.group(4)}s)"
    if io:
        s += f" | io {io.group(3)}/{io.group(2)} cutlp {io.group(4)} {io.group(6)}s"
    if bg:
        s += f" | bg {bg.group(1)}r {float(bg.group(3)):.6g}"
    if res:
        s += f" | {res.group(1)} obj {res.group(2)} bd {float(res.group(3)):.6g} nodes {res.group(4)} {res.group(5)}s rootbd {res.group(6)}"
    print(s)

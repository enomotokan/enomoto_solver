"""ENOMOTO のログ (logs/e_*.log) を 1 行ずつにまとめる。使い方: python3 table.py [名前...]"""
import re, sys, glob, os
D = os.path.dirname(os.path.abspath(__file__))
names = sys.argv[1:] or sorted(os.path.basename(f)[2:-4] for f in glob.glob(f"{D}/logs/e_*.log"))
for nm in names:
    s = open(f"{D}/logs/e_{nm}.log").read()
    rc = re.search(r"Root cuts: objective \S+ -> (\S+), (\d+) cuts in the LP \((\S+)s\)", s)
    rens = re.search(r"RENS: fixing rate (\S+) \(target", s)
    bg = re.search(r"background cuts: \d+ rounds, \d+ cut rows \(was \d+\), obj \S+ -> (\S+) .*?, (\S+)s\)", s)
    rs = len(re.findall(r"^MIP: restart \d", s, re.M))
    # 再スタート前 (最初の求解) の暫定解の最後
    first = s.split("MIP: restart 1")[0]
    incs = re.findall(r"new incumbent (\S+) at node \d+ \((\S+)s\) \[([^\]]+)\]", first)
    good = incs[-1] if incs else None
    tree = re.findall(r"tree search with \d+ threads \((\S+)s\)", s)
    r = re.search(r"RESULT (\S+) (\S+) (\S+) (\S+) (\S+)", s)
    print(f"{nm:6} root {float(rc.group(1)):9.1f}/{rc.group(2)}@{float(rc.group(3)):4.1f}s RENS {rens.group(1) if rens else '-':4} "
          f"bg {float(bg.group(1)) if bg else float('nan'):9.1f} pre-restart inc {float(good[0]) if good else float('nan'):8.0f}@{float(good[1]) if good else 0:4.1f}s[{good[2] if good else ''}] "
          f"rs {rs} tree {'/'.join(tree) or '-'} | {r.group(2)} lb {r.group(3)} nodes {r.group(4)}")

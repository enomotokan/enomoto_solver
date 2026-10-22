"""ログの一覧: python3 summarize.py <ログ>...  最初の根のカットのループのラウンド数・採用本数・LP 反復と、
結果 (暫定解・下界・根の下界)・最初のループの終わりの値と時刻 (求解の始めから)・49324 に初めて届いた所 (ループ番号・時刻)。"""
import json, re, sys

for f in sys.argv[1:]:
    loops, cur = [], None
    res = None
    for l in open(f, errors="replace"):
        m = re.match(r"MIP: cut round (\d+): (\d+) cuts \(of (\d+), sep \S+ LP (\d+) iters\).*obj (\S+) \((\S+)s\)", l)
        if m:
            if cur is None:
                cur = []
            cur.append((int(m.group(2)), int(m.group(4)), float(m.group(5)), float(m.group(6))))
        m = re.match(r"Root cuts: objective (\S+) -> (\S+), (\d+) cuts in the LP \((\S+)s\)", l)
        if m:
            loops.append((cur or [], float(m.group(1)), float(m.group(2)), int(m.group(3)), float(m.group(4))))
            cur = None
        if l.startswith("{"):
            res = json.loads(l)
    name = f.split("/")[-1].replace(".log", "")
    if not loops:
        print(name, "no root loop", res); continue
    r0 = loops[0][0]
    hit = next((f"loop{i} {L[4]:.1f}s" for i, L in enumerate(loops) if L[2] >= 49323.99), "-")
    cuts = "/".join(str(c) for c, *_ in r0[:8])
    its = sum(i for _, i, *_ in r0)
    print(f"{name:28s} inc {res['objective'] if res else None!s:>8.8} bd {res['best_bound'] if res else 0:9.1f} root {res['root_bound'] if res else 0:9.1f} | "
          f"loop0 {len(r0):2d}r {loops[0][2]:8.1f} @{loops[0][4]:5.1f}s iters {its:6d} maxit {max([i for _, i, *_ in r0] or [0]):5d} cuts {cuts} | loops {len(loops)} hit49324 {hit}")

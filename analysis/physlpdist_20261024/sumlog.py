"""ENOMOTO のログの要約: python sumlog.py <log>... 根のカットのラウンド (本数・LP 反復・目的値・時刻)、延長・続き、
根のループの終わり、暫定解の更新 (時刻・出どころ)、根のヒューリスティクスの結果、最後の JSON。-r でラウンドを省く。"""
import json, re, sys

args = [a for a in sys.argv[1:] if a != "-r"]
rounds = "-r" not in sys.argv
for f in args:
    print(f"== {f}")
    for line in open(f, errors="replace"):
        line = line.rstrip()
        m = re.match(r"MIP: cut round (\d+): (\d+) cuts \(of (\d+), sep ([\d.]+)s, LP (\d+) iters\).*obj ([\d.e+-]+) \(([\d.]+)s\)", line)
        if m:
            if rounds:
                r, c, of, sep, it, obj, t = m.groups()
                print(f"  r{r:>2} {c:>4}/{of:<5} sep {sep}s LP {it:>5} it  obj {float(obj):.1f}  @{t}s")
            continue
        if rounds and re.search(r"still improving|LP continued|LP hit the time cap|did not move", line):
            print("   ", line[5:120])
        elif re.search(r"^Root cuts|restart|new incumbent|root heuristic .*: found|parallel sub-MIP \(|sub-MIP: \d+ free|RINS \(|RENS \(|after root heuristics", line):
            print("   ", line[:170])
        elif line.startswith("{"):
            d = json.loads(line)
            print(f"  -> {d['status']} obj {d['objective']} bound {d['best_bound']} root {d['root_bound']} nodes {d['nodes']} time {d['time']:.1f}")

# ENOMOTO のログの要点 (時刻は再スタート後も通しの秒に直す): python summ.py <log>...
import re, sys
for fn in sys.argv[1:]:
    off = 0.0; last_t = 0.0; out = []; prog = None
    for ln in open(fn, errors="replace"):
        m = re.search(r"Restart \d+: .*\(([\d.]+)s\)", ln)
        if m: off += float(m.group(1)); out.append(f"  restart @{off:.1f}s"); continue
        m = re.search(r"Root cuts: objective (\S+) -> (\S+), (\d+) cuts.*\(([\d.]+)s\)", ln)
        if m: out.append(f"  root cuts {float(m.group(1)):.1f} -> {float(m.group(2)):.1f} ({m.group(3)} cuts) @{off+float(m.group(4)):.1f}s"); continue
        m = re.search(r"new incumbent (\S+) at node (\d+) \(([\d.]+)s\) \[(.*)\]", ln)
        if m and m.group(4) != "tree" or (m and float(m.group(3))>0):
            out.append(f"  inc {float(m.group(1)):.0f} @{off+float(m.group(3)):.1f}s [{m.group(4)}]"); continue
        m = re.match(r"\s+([\d.]+)s\s+(\d+)\s+(\d+)\s+(\S+)\s+(\S+)\s+(\S+)", ln)
        if m: prog = (off+float(m.group(1)), int(m.group(2)), float(m.group(4)), m.group(5))
        if ln.startswith("RESULT"):
            r = re.search(r"'status': '(\w+)', 'objective': ([\d.e+-]+)", ln)
            out.append(f"  end: {r.group(1)} obj {float(r.group(2)):.0f} wall {ln.split()[2]}")
    if prog: out.append(f"  last progress @{prog[0]:.1f}s nodes {prog[1]} lb {prog[2]:.1f} ub {prog[3]}")
    print(fn); print("\n".join(out))

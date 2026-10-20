"""ENOMOTO / HiGHS のログの要約 (根の下界・暫定解の推移・ノード数・下界の推移)。使い方: python summarize.py <log>..."""
import re, sys
def eno(path):
    L = open(path, errors="replace").read().splitlines()
    out = []
    for l in L:
        if l.startswith("Root cuts:") or l.startswith("Restart") or "presolve: rows" in l and "MIP:" in l:
            out.append("  " + l[:120])
    inc = [re.search(r"new incumbent (\S+) .*?\[(.*)\]", l) for l in L]
    st = [l for l in L if re.match(r"MIP: +[0-9.]+s nodes", l)]
    inc_t = []
    last_t = 0.0
    for l in L:
        m = re.match(r"MIP: +([0-9.]+)s nodes +(\d+) open +(\d+) lb (\S+) ub (\S+)", l)
        if m: last_t = float(m.group(1))
        m2 = re.search(r"new incumbent (\S+) at node (\d+) \(([0-9.]+)s\) \[(.*)\]", l)
        if m2: inc_t.append(f"{float(m2.group(1)):.2f}@{m2.group(3)}s({m2.group(4)[:12]})")
    out.append("  incumbents: " + " ".join(inc_t[:12]) + (" ..." if len(inc_t) > 12 else "") + (" last " + inc_t[-1] if len(inc_t) > 12 else ""))
    marks = [10, 20, 30, 40, 50, 60, 90, 120, 180, 240, 300]
    prog = []
    gl = [re.match(r"^ +([0-9.]+)s +(\d+) +(\d+) +(\S+) +(\S+) +(\S+)%?$", l) for l in L]
    gl = [m for m in gl if m]
    for m in gl:
        t = float(m.group(1))
        while marks and t >= marks[0]:
            prog.append(f"{marks.pop(0)}s:n{m.group(2)} lb{float(m.group(4)):.3f}")
    out.append("  progress: " + " | ".join(prog))
    res = [l for l in L if l.startswith("RESULT")]
    if res:
        r = res[-1]
        m = re.search(r"RESULT (\S+) \{'status': '(\w+)', 'objective': (\S+),", r)
        bb = re.search(r"'best_bound': (\S+?),", r); nd = re.search(r"'nodes': (\d+)", r)
        out.append(f"  RESULT t={float(m.group(1)):.1f} {m.group(2)} obj={m.group(3)} bound={bb.group(1) if bb else '?'} nodes={nd.group(1) if nd else '?'}")
    return out
for p in sys.argv[1:]:
    print(p); print("\n".join(eno(p)))

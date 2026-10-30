"""ログの要約: ENOMOTO は区間 (再スタート) ごとの時刻を足して経過時間に直し、暫定解・根の下界・再スタートを並べる。HiGHS は表の行を間引いて出す。"""
import re, sys, json
from pathlib import Path
D = Path(__file__).resolve().parent
def ours(p):
    base = 0.0; last = 0.0; ev = []
    for ln in p.read_text().splitlines():
        m = re.search(r"\(([\d.]+)s\)", ln)
        t = float(m.group(1)) if m else None
        if "MIP: restart " in ln and t is not None:
            ev.append(f"{base+t:6.1f} {ln[5:110]}"); base += t; last = 0; continue
        if t is not None:
            last = t
        if "new incumbent" in ln and "[tree]" not in ln:
            ev.append(f"{base+(t or last):6.1f} {ln[5:]}")
        elif re.search(r"root LP [-\d]|root LP after|cut round \d+:.*obj|root bound|after root heuristics|nodes .*bound|early restart:", ln):
            ev.append(f"{base+(t or last):6.1f} {ln[5:150]}")
        elif ln.startswith("{"):
            ev.append(ln)
    # cut round は区間ごとに最後の 1 行だけ残す
    out = []
    for i, e in enumerate(ev):
        if "cut round" in e and i + 1 < len(ev) and "cut round" in ev[i + 1]:
            continue
        out.append(e)
    return out
def highs(p):
    out = []
    for ln in p.read_text().splitlines():
        if re.match(r"\s*[A-Za-z ]?\s+\d+\s+\d+\s+\d+\s+[\d.]+%", ln) or "Solving report" in ln or re.search(r"Status|Primal bound|Dual bound|Gap|Timing|Nodes|WALL|Presolving model|Presolve reductions", ln):
            out.append(ln.rstrip())
    return out
for f in sorted(D.glob(sys.argv[1] if len(sys.argv) > 1 else "*.log")):
    print("=" * 20, f.name)
    print("\n".join(highs(f) if "_highs_" in f.name else ours(f)))

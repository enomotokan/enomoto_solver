import sys,re,glob
for n in sys.argv[1:]:
    out=open(n+".out",errors="replace").read().splitlines()
    prog=[l for l in out if re.match(r"\s+[\d.]+s\s+\d+",l)]
    res=[l for l in out if l.startswith("RESULT")]
    t=res[-1].split()[1] if res else "?"
    last=prog[-1].split() if prog else []
    rows=[]
    try:
        for l in open(n+".err",errors="replace"):
            if l.startswith("ENODE"):
                m=re.search(r"rows=(\d+)",l); rows.append(int(m.group(1)))
    except FileNotFoundError: pass
    rows.sort()
    rr=f"rows med {rows[len(rows)//2]} [{rows[0]},{rows[-1]}]" if rows else ""
    restart=[l for l in out if l.startswith("Root cuts") or l.startswith("Restart")]
    print(f"{n:12s} {t:8s} nodes {last[1] if last else '?':>6s} gap {last[-1] if last else '?':>6s} | {rr}")

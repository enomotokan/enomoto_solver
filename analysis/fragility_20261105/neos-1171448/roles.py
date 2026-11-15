"""各回の、ヒューリスティクス役に就いた時刻 (木の探索の始まりから) と、−309 を見つけた時刻・出どころ。使い方: python roles.py [接頭辞 ...]"""
import glob, json, os, re, sys
d = os.path.dirname(os.path.abspath(__file__))
pre = sys.argv[1:] or [""]
def key(f):
    t = os.path.basename(f)[4:-4]; m = re.match(r"([a-z_]*?)_?(\d+)$", t)
    return (m.group(1), int(m.group(2))) if m else (t, 0)
for f in sorted(glob.glob(os.path.join(d, "log_*.txt")), key=key):
    tag = os.path.basename(f)[4:-4]
    if not any(tag.startswith(p) for p in pre): continue
    L = open(f).read().splitlines()
    try: r = json.loads(L[-1])
    except Exception: continue
    ts = [float(re.search(r"\((\S+)s\)", l).group(1)) for l in L if l.startswith("MIP: tree search with")]
    role = None
    for l in L:
        m = re.match(r"FRAG: alns tid \d+ role true t (\S+) arm \d dt (\S+)", l)
        if m: role = float(m.group(1)) - float(m.group(2)); break
    mid = any(l.startswith("FRAG: role entry mid-plunge") for l in L)
    inc = [re.search(r"new incumbent (\S+) at node (\d+) \((\S+)s\) \[(.*)\]", l) for l in L]
    inc = [m for m in inc if m and float(m.group(1)) < -308.99]
    best = f"{inc[0].group(3)}s node {inc[0].group(2)} [{inc[0].group(4)}]" if inc else "-"
    t0 = f"{ts[0]:.1f}" if ts else "-"
    rs = f"{role:.1f}" + (" (潜りの途中)" if mid else "") if role is not None else "-"
    print(f"| {tag} | {r['time']:.1f} | {r['nodes']} | {t0} | {rs} | {best} |")

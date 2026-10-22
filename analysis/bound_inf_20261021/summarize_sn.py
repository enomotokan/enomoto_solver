"""snA/snB/snC のログの要約: 結果・従の根 LP・スレッドごとのノード LP の反復/LP。"""
import glob, json, re, sys
from collections import defaultdict
rows = defaultdict(list)
for f in sorted(glob.glob(sys.argv[1] if len(sys.argv) > 1 else "logs/*_sn[ABC]_*.log")):
    m = re.search(r"logs/(.*)_(sn[ABC])_(\d+)\.log", f)
    prob, var, k = m.groups()
    txt = open(f).read()
    res = re.search(r"RESULT (.*)", txt)
    r = json.loads(res.group(1)) if res else {}
    tree = re.search(r"tree search with 4 threads \(([\d.]+)s\)", txt)
    root = re.findall(r"tid (\d) root LP (\w+) from the snapshot basis (\w+) \(dse (\w+)\): (\d+) iters, ([\d.]+)s .*objective ([-\d.e+]+), (\d+) rows", txt)
    th = re.findall(r"node diag \(thread (\d)\): nodes (\d+) node LPs (\d+) .* iters (\d+) \(([\d.]+)/LP\)", txt)
    mast = re.search(r"^MIP: node diag: nodes (\d+) node LPs (\d+) .* iters (\d+) \(([\d.]+)/LP\)", txt, re.M)
    wn = re.findall(r"worker ended: status (\w+) objective (\S+) bound (\S+) nodes (\d+)", txt)
    rows[(prob, var)].append(dict(k=k, status=r.get("status"), obj=r.get("objective"), t=r.get("time"), nodes=r.get("nodes"),
        tree=tree.group(1) if tree else "-", root=[(a[1][:3], a[4], a[5], a[6], a[7]) for a in root][:1],
        ipl=[a[4] for a in th], mipl=mast.group(4) if mast else "-", wnodes=[a[3] for a in wn]))
for (prob, var), lst in sorted(rows.items()):
    for d in lst:
        print(f"{prob:10} {var} #{d['k']} {str(d["status"]):10} obj {d['obj']} t {str(d["t"]):6} nodes {str(d["nodes"]):6} tree@{d['tree']:>5}s root {d['root']} iters/LP main {d['mipl']} workers {d['ipl']} wnodes {d['wnodes']}")

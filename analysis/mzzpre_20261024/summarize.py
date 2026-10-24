"""ENOMOTO のログを 1 行に要約する: python summarize.py logs/<名前>.log ...
列: 最初のカットのループの後の下界・再スタートの回数 (各回の時刻と新たに固定した列)・再スタートごとの根の最終・暫定解の出どころ・最後の JSON"""
import json, re, sys
for f in sys.argv[1:]:
    L = open(f, errors="replace").read().splitlines()
    cut1 = None; rs = []; roots = []; incs = []; res = {}
    for l in L:
        m = re.match(r"MIP: cut loop done: .*obj (\S+) -> (\S+)", l)
        if m:
            roots.append(float(m.group(2)))
        m = re.match(r"MIP: restart (\d+) \((\d+) of \d+ integer columns newly fixed.*\) \(([\d.]+)s\)", l)
        if m:
            rs.append(f"{m.group(3)}s/{m.group(2)}")
        m = re.match(r"MIP: new incumbent (\S+) at node \d+ \(([\d.]+)s\) \[(.*)\]", l)
        if m:
            incs.append((float(m.group(1)), m.group(3)))
        if l.startswith("{"):
            res = json.loads(l)
    best = min(incs)[1] if incs else "-"
    print(f"{f.split('/')[-1]:32s} roots {['%.0f' % r for r in roots]} restarts {rs} obj {res.get('objective')} bound {res.get('best_bound') and round(res['best_bound'],1)} nodes {res.get('nodes')} status {res.get('status')} best-from [{best}]")

"""logs/<問題>_<名前>_<n>.log から表を作る: python table.py <名前> [n]"""
import json, re, sys, glob, os
name = sys.argv[1]
runs = sys.argv[2:] or None
rows = []
for f in sorted(glob.glob(f'logs/*_{name}_*.log')):
    m = re.match(rf'logs/(.+)_{re.escape(name)}_(\d+)\.log', f)
    p, n = m.group(1), m.group(2)
    if runs and n not in runs:
        continue
    txt = open(f).read().splitlines()
    try:
        js = json.loads(txt[-1])
    except Exception:
        js = {}
    fj = next((l for l in txt if 'FJ best point' in l), '')
    mm = re.search(r'found (\w+), least violated rows (-?\d+), kept point (Some\(\((\d+), ([^)]+)\)\)|None), recount (Some\(\((\d+), ([^)]+)\)\)|None), incumbent (\w+) \(([\d.]+)s\)', fj)
    ls = next((re.search(r'least violation (\S+)', l).group(1) for l in txt if 'least violation' in l), '')
    inc = next((l for l in txt if 'new incumbent' in l), '')
    mi = re.search(r'new incumbent (\S+) at node (\d+) \(([\d.]+)s\) \[(.*)\]', inc)
    fr = [l for l in txt if 'FJ repair' in l]
    rows.append(dict(p=p, n=n,
        fjfound=mm.group(1) if mm else '-', fjmin=mm.group(2) if mm else '-',
        rec=(mm.group(7), mm.group(8)) if mm and mm.group(7) else None, fjt=mm.group(10) if mm else '-',
        ls=ls, first=(mi.group(1), mi.group(3), mi.group(4)) if mi else None,
        js=js, fr=fr))
for r in rows:
    js = r['js']
    first = f"{float(r['first'][0]):.6g} @{float(r['first'][1]):.1f}s [{r['first'][2]}]" if r['first'] else '-'
    rec = f"{r['rec'][0]} / {float(r['rec'][1]):.3g}" if r['rec'] else '-'
    obj = js.get('objective'); bb = js.get('best_bound')
    fin = f"{js.get('status')} {obj if obj is None else round(obj, 6)} (bound {bb if bb is None else round(bb, 4)}, {js.get('time', 0):.1f}s)"
    frs = ' / '.join(x.split('MIP: ')[-1][:160] for x in r['fr'])
    print(f"| {r['p']} | {r['n']} | {r['fjfound']} | {r['fjmin']} | {rec} | {r['fjt']} | {r['ls']} | {first} | {fin} |" + (f" {frs} |" if frs else ''))

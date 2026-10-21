"""ログを 1 行ずつまとめる。使い方: python3 table.py [問題...] (省略時は logs/ の全部)。
列: 結果 (解けた秒 / 60 秒の暫定解)、再スタートの回数、木に入った時刻 (再スタートを通した通算)、最初の根のカット後の下界、
最初の背景のカットのラウンド数と下界、FACETIME (最初の根: 分離のうち面の時間 / 分離 / LP)。誤った最適値は「誤」"""
import re, sys, glob, os, collections
OPT = {"mc11": 11689, "mzzv42z": -20540, "h80x6320d": 6382.09905, "noswot": -41, "qnet1": 16029.692681, "gen-ip002": -4783.733392,
       "gen-ip054": 6840.966, "30n20b8": 302, "neos-1456979": 176, "timtab1": 764772, "gr4x6": 202.35, "rout": 1077.56,
       "mik-250-20-75-4": -52301, "flugpl": 1201500, "gt2": 21166, "blend2": 7.598985, "roll3000": 12890, "uccase12": 11507.405,
       "neos-911970": 54.76, "neos-960392": -238, "beasleyC3": 754, "drayage-25-23": 101282.647, "drayage-100-23": 103333.874,
       "physiciansched6-2": 49324, "app1-1": -3, "nursesched-sprint02": 58, "hypothyroid-k1": -2851, "ns1116954": 0,
       "neos-950242": 4, "neos-2978193-inde": -2.388061686}
def parse(f):
    s = open(f).read()
    m = re.search(r"RESULT (\S+) (\S+) (\S+) (\S+) (\S+)", s)
    r = {}
    if not m:
        return None
    st, obj, bd, nodes, t = m.groups()
    obj = None if obj == "None" else float(obj)
    p = os.path.basename(f).rsplit("_", 2)[0]
    if st == "optimal":
        cell = f"{float(t):.1f}s"
        o = OPT.get(p)
        if o is not None and obj is not None and abs(obj - o) > 1e-6 * max(1, abs(o)) + 1e-6:
            cell = f"誤{obj:g}({t}s)"
    else:
        cell = f"{obj:g}" if obj is not None else "解なし"
    off = 0.0; tree = None; rs = 0
    for line in s.splitlines():
        mm = re.match(r"MIP: restart (\d+) .*\(([\d.]+)s\)$", line)
        if mm:
            rs = int(mm.group(1)); off += float(mm.group(2))
        mm = re.match(r"MIP: tree search with \d+ threads \(([\d.]+)s\)", line)
        if mm:
            tree = off + float(mm.group(1))
    rc = re.search(r"Root cuts: objective \S+ -> (\S+), (\d+) cuts", s)
    bg = re.search(r"background cuts: (\d+) rounds, \d+ cut rows \(was \d+\), obj \S+ -> (\S+)", s)
    ft = re.search(r"FACETIME root restarts 0 rounds (\d+): sep ([\d.]+)s \(face ([\d.]+)s", s)
    ftl = re.search(r"FACETIME root restarts 0 .*LP ([\d.]+)s, loop ([\d.]+)s", s)
    fb = re.search(r"FACETIME bg restarts 0 rounds (\d+): sep ([\d.]+)s \(face ([\d.]+)s.*LP ([\d.]+)s, loop ([\d.]+)s", s)
    return dict(p=p, cell=cell, nodes=nodes, rs=rs, tree=tree,
                root=f"{float(rc.group(1)):.6g}/{rc.group(2)}" if rc else "-",
                bg=f"{bg.group(1)}r {float(bg.group(2)):.6g}" if bg else "-",
                ft=f"{ft.group(1)}r face {ft.group(3)}/sep {ft.group(2)}/LP {ftl.group(1)} loop {ftl.group(2)}" if ft else "-",
                fb=f"{fb.group(1)}r face {fb.group(3)}/sep {fb.group(2)}/LP {fb.group(4)}" if fb else "-")
files = sorted(glob.glob(os.path.join(os.environ.get("LOGS") or os.path.join(os.path.dirname(__file__) or ".", "logs"), "*.log")))
want = sys.argv[1:]
rows = collections.defaultdict(list)
for f in files:
    b = os.path.basename(f)[:-4]
    p, c, k = b.rsplit("_", 2)
    if want and p not in want:
        continue
    d = parse(f)
    if d:
        rows[(p, c)].append((k, d))
for (p, c), lst in sorted(rows.items()):
    for k, d in sorted(lst):
        tr = f"{d['tree']:.1f}" if d['tree'] is not None else "-"
        print(f"{p:18s} {c:6s} {k:2s} {d['cell']:>16s} rs {d['rs']} tree {tr:>5s} nodes {d['nodes']:>7s} root {d['root']:>14s} bg {d['bg']:>14s} | {d['ft']} | bg {d['fb']}")

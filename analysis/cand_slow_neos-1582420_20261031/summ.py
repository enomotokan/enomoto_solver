"""設定ごと・根の道 (A/B) ごとの時間 (秒) と最初の区間の根のヒューリスティクスの秒を出す。時間切れは T (60 として平均)。
使い方: python summ.py [問題] [設定,...]"""
import re, sys, glob, collections
from pathlib import Path
D = Path(__file__).resolve().parent
prob = sys.argv[1] if len(sys.argv) > 1 else "neos-1582420"
want = sys.argv[2].split(",") if len(sys.argv) > 2 else None
rows = collections.defaultdict(list)
for f in sorted(glob.glob(str(D / "logs" / f"{prob}_*.log")), key=lambda x: int(x.rsplit("_", 1)[1][:-4])):
    c = Path(f).stem[len(prob) + 1:].rsplit("_", 1)[0]
    if want and c not in want:
        continue
    s = open(f).read()
    st = re.findall(r'"status": "(\w+)"', s); t = re.findall(r'"time": ([\d.]+)', s)
    if not t:
        continue
    t = 60.0 if st[-1] != "optimal" or float(t[-1]) > 60 else float(t[-1])
    cl0 = re.search(r"cut loop done: .*-> ([\d.e+-]+)", s)
    path = {"8.9251127551e1": "A", "8.9261487563e1": "B"}.get(cl0.group(1) if cl0 else "", "?")
    rh = re.search(r"after root heuristics: incumbent (\S+) \(([\d.]+)s\)", s)
    rs = len(re.findall(r"^MIP: restart \d+ \(", s, re.M))
    rows[c].append((t, path, float(rh.group(2)) if rh else -1, rs, st[-1] == "optimal"))
order = want or sorted(rows, key=lambda c: (c not in ("def", "cand"), c))
for c in order:
    r = rows.get(c, [])
    if not r:
        continue
    mean = sum(x[0] for x in r) / len(r)
    byp = {p: [x[0] for x in r if x[1] == p] for p in "AB?"}
    pm = "  ".join(f"{p} {sum(v)/len(v):5.1f} ({len(v)})" for p, v in byp.items() if v)
    det = " ".join(f"{x[0]:.1f}{'' if x[4] else 'T'}{x[1]}/h{x[2]:.0f}/r{x[3]}" for x in r)
    print(f"{c:10s} 平均 {mean:5.1f} ({len(r)}回)  {pm:32s} | {det}")

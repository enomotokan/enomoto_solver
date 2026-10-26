"""設定ごとの集計: 根のループの最後の下界 (最初の求解)、bg の後の根の下界 (rootbd)、60 秒で解けた回数と時間・最終の下界。
使い方: python3 table.py logs/*.log"""
import re, sys, collections
rows = collections.defaultdict(list)
for f in sys.argv[1:]:
    t = open(f, errors="replace").read()
    m = re.match(r"(.*)_([a-z0-9]+)_(\d+)$", f.split("/")[-1][:-4])
    if not m:
        continue
    p, c, k = m.groups()
    rc = re.search(r"Root cuts: objective \S+ -> ([-0-9.e+]+)", t)
    res = re.search(r"RESULT (\S+) (\S+) (\S+) (\d+) ([0-9.]+) (\S+)", t)
    if not res:
        continue
    rows[(p, c)].append((float(rc.group(1)) if rc else float("nan"), float(res.group(6)) if res.group(6) != "None" else float("nan"), res.group(1) == "optimal", float(res.group(5)), float(res.group(3)), res.group(2)))
for (p, c), v in sorted(rows.items()):
    solved = [x[3] for x in v if x[2]]
    fmt = lambda a: "/".join(f"{x:.6g}" for x in a)
    print(f"| {p} | {c} | {len(v)} | {fmt([x[0] for x in v])} | {fmt([x[1] for x in v])} | {len(solved)}/{len(v)} {fmt(solved)} | {fmt([x[4] for x in v])} |")

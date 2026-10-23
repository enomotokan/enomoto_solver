"""logs/ の結果を表にする: python -I table.py [名前 ...]"""
import json, re, sys, glob, os, collections
names = sys.argv[1:] or ["def", "lm", "lmc"]
rows = collections.defaultdict(dict)
for f in sorted(glob.glob(os.path.join(os.path.dirname(os.path.abspath(__file__)), "logs", "*.log"))):
    b = os.path.basename(f)[:-4]
    m = re.match(r"(.+)_([a-z0-9]+)_(\d+)$", b)
    if not m or m.group(2) not in names: continue
    p, cfg, k = m.groups()
    L = open(f, errors="replace").read().splitlines()
    try: r = json.loads(L[-1])
    except Exception: continue
    inc = next((l for l in L if "new incumbent" in l), "")
    mi = re.search(r"new incumbent (\S+) at node \d+ \(([\d.]+)s\) \[(.*)\]", inc)
    first = f"{float(mi.group(1)):.6g}・{mi.group(2)} 秒 ({mi.group(3)})" if mi else "解なし"
    fjv = [int(x) for x in re.findall(r"FJ2.*least violated rows (\d+)", "\n".join(L))]
    fjf = any(re.search(r"FJ2.*found true", l) for l in L)
    lm = next((l for l in L if l.startswith("MIP: Local-MIP:")), "")
    lmv = re.search(r"least violated rows (\d+)", lm)
    lmf = re.search(r"found (\S+) \(first at (\S+)s", lm)
    lmd = f"{lmv.group(1)} 行" if lmv else "-"
    if lmf and lmf.group(1) != "none": lmd = f"解 {float(lmf.group(1)):.6g} ({lmf.group(2).replace('Some(','').rstrip(')')} 秒)"
    fin = r.get("objective"); st = r.get("status")
    fins = ("解なし" if fin is None else f"{fin:.6g}") + (" 最適" if st == "optimal" else "")
    rows[p][(cfg, int(k))] = (first, fins, ("解あり" if fjf else (f"{min(fjv)} 行" if fjv else "-")), lmd)
print("| 問題 | 設定 | 回 | 最初の暫定解 (出どころ) | 最終 | FJ の最少違反行 | Local-MIP |")
print("|---|---|---|---|---|---|---|")
for p in rows:
    for (cfg, k) in sorted(rows[p], key=lambda t: (names.index(t[0]), t[1])):
        a = rows[p][(cfg, k)]
        print(f"| {p} | {cfg} | {k} | {a[0]} | {a[1]} | {a[2]} | {a[3]} |")

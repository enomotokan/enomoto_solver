"""run.py のログを 1 本 1 行にまとめる (2026-11-01)。
使い方: python analyze.py [-v]  (-v で暫定解の並びとカットのループの止めの行も出す)
列: 状態・時間・最後の暫定解・最後の下界・ノード | 根の LP の秒・カットのラウンド数・カットのループの終わりの時刻・カットの後の下界・
根のヒューリスティクスの終わりの時刻と暫定解 (根の終わり)・木に入る時刻・再スタート・最適値の解を見つけた時刻と出どころ。"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
L = D / "logs"
V = "-v" in sys.argv
T = re.compile(r"\((\d+\.\d+)s\)")
OPT = {"neos-1171448": -309.0, "neos-1171737": -195.0}


def parse(f):
    r = dict(name=f.stem, inc=[], rounds=0, stops=[], restarts=0)
    for line in open(f, errors="replace"):
        if line.startswith("{") and '"status"' in line:
            r["res"] = json.loads(line); continue
        m = re.match(r"MIP: root LP (\S+) \((\d+) iters, (\S+)s\)", line)
        if m and "rootlp" not in r:
            r["rootlp"] = float(m[3]); r["rootlpobj"] = float(m[1])
        m = re.match(r"MIP: cut round (\d+): \d+ cuts .* obj (\S+) \((\d+\.\d+)s\)", line)
        if m and "cutdone" not in r:
            r["rounds"] = int(m[1]) + 1; r["cutobj"] = float(m[2]); r["lastT"] = float(m[3])
        if "cutdone" not in r and (re.match(r"MIP: cut (round \d+: (the cut LP hit|LP hit|LP status|the LP with))", line) or re.match(r"MIP: cut loop: ", line)):
            r["stops"].append(line.strip())
        if line.startswith("MIP: cut loop done"):
            r["cutdone"] = r.get("lastT"); m = re.search(r"-> (\S+)", line); r["cutobj"] = float(m[1].rstrip(","))
        if line.startswith("MIP: after root heuristics"):
            t = float(T.search(line)[1])
            if t >= r.get("heurdone", 0):
                r["heurdone"] = t; r["rootinc"] = line.split("incumbent ")[1].split(" (")[0].replace("Some(", "").rstrip(")")
        if line.startswith("MIP: tree search with"):
            r.setdefault("tree", float(T.search(line)[1]))
        if re.match(r"MIP: restart \d+: pseudocost observations", line):
            r["restarts"] += 1
        m = re.match(r"MIP: new incumbent (\S+) at node (\d+) \((\S+)s\) \[(.*)\]", line)
        if m:
            r["inc"].append((float(m[3]), float(m[1]), m[4]))
    return r


def key(p):
    m = re.match(r"(.*)_(def|cs2)_r(\d+)", p.stem)
    return (m[1], int(m[3]), m[2] != "def")


for f in sorted(L.glob("*.log"), key=key):
    r = parse(f); res = r.get("res", {})
    prob = key(f)[0]
    st = {"optimal": "opt", "time_limit": "TL"}.get(res.get("status"), res.get("status"))
    best = [x for x in r["inc"] if x[1] <= OPT[prob] + 1e-3]
    print(f"{r['name']:24s} {st:4s} {res.get('time', 0):5.1f}s obj {res.get('objective', float('nan')):10.4f} lb {res.get('bound', float('nan')):9.3f} nodes {res.get('nodes', 0):5d}"
          f" | rootLP {r.get('rootlpobj', 0):.2f} {r.get('rootlp', 0):4.2f}s rounds {r['rounds']:2d} cutdone {r.get('cutdone') or 0:5.2f} cutlb {r.get('cutobj', 0):.3f}"
          f" heur {r.get('heurdone', 0):5.2f} (inc {r.get('rootinc')}) tree {r.get('tree', 0):5.2f} rs {r['restarts']}"
          f" | opt at {f'{best[0][0]:.2f}s [{best[0][2]}]' if best else '-'}")
    if V:
        for s in r["stops"]:
            print("     " + s[:170])
        for t, z, s in r["inc"]:
            print(f"     inc {t:6.2f}s {z:11.4f} [{s}]")

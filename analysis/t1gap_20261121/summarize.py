"""logs/ を読んで問題ごとに並べる: python summarize.py [問題...]
当方: 暫定解の時刻・値・出どころ ([...])、根 LP・切除後の下界、再スタート、進み (時刻ごとの nodes/lb/ub)、最終のノード数。
SCIP: 根の下界 (ノード 1 の最後の行)、暫定解の行 (先頭の文字 = ヒューリスティクスの印)、最終のノード数。"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
L = D / "logs"
INC = re.compile(r"MIP: new incumbent (\S+) at node (\d+) \(([\d.]+)s\)(?: \[(.*)\])?")
PRO = re.compile(r"MIP:\s+([\d.]+)s nodes\s+(\d+) open\s+(\d+) lb (\S+) ub (\S+)")
def ours(f, marks=(30, 60, 120, 300, 600, 1200, 2400)):
    inc, pro, root = [], [], {}
    for l in f.open(errors="replace"):
        m = INC.search(l)
        if m: inc.append((float(m[3]), float(m[1]), int(m[2]), m[4] or "")); continue
        m = PRO.search(l)
        if m: pro.append((float(m[1]), int(m[2]), int(m[3]), m[4], m[5])); continue
        if "MIP: root LP " in l and "iters" in l and "root" not in root:
            m = re.search(r"root LP (\S+) \(", l); root["root"] = m and m[1]
        if "cut loop done" in l: root["cut"] = l.split("->")[-1].strip()
        if "background cuts:" in l and "rounds," in l and "obj" in l: root["bg"] = l.split("obj")[-1].strip()[:60]
        if "restart" in l.lower() and ("restarting" in l.lower() or "restart:" in l.lower()): root.setdefault("restarts", []).append(l.strip()[5:90])
        if "tree search threads:" in l: root["threads"] = l.strip()[5:]
        if "SUBFIX summary" in l: root["subfix"] = l.strip()[5:]
    print(f"  root: {root.get('root')} cut-> {root.get('cut')} bg-> {root.get('bg')}")
    for r in root.get("restarts", [])[:4]: print("   ", r)
    if "threads" in root: print("  ", root["threads"][:200])
    print(f"  incumbents ({len(inc)}):")
    for t, v, n, s in inc: print(f"    {t:8.2f}s {v:.6g} node {n} [{s}]")
    out, j = [], 0
    for mk in marks:
        c = [p for p in pro if p[0] <= mk]
        if c and (not out or c[-1] != out[-1]): out.append(c[-1])
    if pro and pro[-1] not in out: out.append(pro[-1])
    for p in out: print(f"    t {p[0]:8.1f} nodes {p[1]:8d} open {p[2]:6d} lb {p[3]} ub {p[4]}")
def scip(f):
    rows, root, last = [], None, None
    for l in f.open(errors="replace"):
        if "|" in l and re.match(r"^[ a-zA-Z*]\s*[\d.]+s\|", l):
            c = [x.strip() for x in l.split("|")]
            t = c[0][1:].strip(); mark = l[0]
            if c[1] == "1": root = c[-4]
            last = c
            if mark != " ": rows.append((mark, t, c[1], c[-3], c[5] if len(c) > 5 else ""))
        if l.startswith("Solving Nodes"): print("  ", l.strip())
        if l.startswith("Solving Time"): print("  ", l.strip())
    print(f"  root dual (last node-1 row): {root}; last row: t {last and last[0]} nodes {last and last[1]} dual {last and last[-4]} primal {last and last[-3]}")
    for r in rows: print(f"    {r[0]} {r[1]:>8} node {r[2]:>6} primal {r[3]} heur {r[4]}")
res = [json.loads(l) for l in (D / "res.jsonl").open()]
probs = sys.argv[1:] or list(dict.fromkeys(r["problem"] for r in res))
for p in probs:
    print("=" * 20, p)
    for r in res:
        if r["problem"] != p: continue
        x = r["result"]
        print(f"- {r['cfg']} {r['tl']}: {x.get('status')} t {x.get('time')} obj {x.get('objective')} bd {x.get('bound')} nodes {x.get('nodes')} root {x.get('root_bound')} first {x.get('first_solution_time')} {x.get('first_solution_objective')}")
        f = L / f"{p}_{r['cfg']}_{r['tl']}.log"
        if f.exists(): (scip if r["cfg"] == "scip" else ours)(f)

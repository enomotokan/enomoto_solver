"""neos-1171448 の def・trim3 のログ (run.py) を 1 本 1 行にまとめる (2026-11-01)。
使い方: python analyze.py [ログのディレクトリ (既定 logs)] [-v]  (-v で暫定解の時刻の並びと RENS・RINS の行も出す)
列: 状態・時間・最適値・最後の下界・根の LP の秒 (行の時刻)・カットのループの終わり・根のヒューリスティクスの終わり・
木に入った時刻・再スタートの回数・木のノード数・最初の解の時刻・−309 の解の時刻と出どころ。"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
L = D / (sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("-") else "logs")
V = "-v" in sys.argv
T = re.compile(r"\((\d+\.\d+)s\)")


def parse(f):
    r = dict(name=f.stem, inc=[], subs=[], restarts=0)
    for line in open(f, errors="replace"):
        if line.startswith("{") and '"status"' in line:
            r["res"] = json.loads(line)
            continue
        if line.startswith("MIP: root LP -"):
            m = re.search(r"\((\d+) iters, (\S+)s\)", line); r.setdefault("rootlp", float(m[2]))
        elif line.startswith("MIP: cut round") and "cut_last" not in r:
            pass
        if line.startswith("MIP: cut loop done"):
            r.setdefault("cutdone", r.get("lastT"))
        m = T.search(line)
        if m and line.startswith("MIP: cut round") and "cutdone" not in r:
            r["lastT"] = float(m[1])
        if line.startswith("MIP: after root heuristics"):
            t = float(T.search(line)[1])
            if t >= r.get("heurdone", 0):  # VERBOSE ではサブ MIP の中の同じ行も出る。外側のものは時刻が一番大きい
                r["heurdone"] = t; r["rootinc"] = line.split("incumbent ")[1].split(" (")[0]
        if line.startswith("MIP: background cuts:") and "not used" in line or "background cuts: 6 rounds" in line and "used" in line:
            r["bg"] = line.strip()[len("MIP: background cuts: "):][:80]
        if line.startswith("MIP: tree search with"):
            r.setdefault("tree", float(T.search(line)[1]))
        if re.match(r"MIP: (early )?restart \d+:", line) or line.startswith("MIP: early restart"):
            pass
        if re.match(r"MIP: restart \d+: pseudocost observations", line):
            r["restarts"] += 1
        m = re.match(r"MIP: new incumbent (\S+) at node (\d+) \((\S+)s\) \[(.*)\]", line)
        if m:
            r["inc"].append((float(m[3]), float(m[1]), m[4]))
        if re.match(r"MIP:   (parallel sub-MIP|sub-MIP:|RENS|RINS)", line) or line.startswith("MIP: root heuristic RENS") or line.startswith("MIP: root heuristic RINS") or "root heuristic RENS (more nodes)" in line:
            r["subs"].append(line.strip())
        m = re.match(r"MIP: +(\d+\.\d+)s nodes +(\d+) open +(\d+) lb (\S+)", line)
        if m:
            r["lastlog"] = (float(m[1]), int(m[2]), m[4])
    return r


rows = []
for f in sorted(L.glob("*.log"), key=lambda p: (int(re.search(r"_r(\d+)", p.stem)[1]), p.stem)):
    r = parse(f); res = r.get("res", {})
    st = {"optimal": "opt", "time_limit": "TL"}.get(res.get("status"), res.get("status"))
    best = [x for x in r["inc"] if x[1] <= -308.999]
    first = r["inc"][0] if r["inc"] else None
    rows.append((r, res))
    print(f"{r['name']:9s} {st:4s} {res.get('time', 0):5.1f}s obj {res.get('objective', float('nan')):8.2f} lb {res.get('bound', float('nan')):8.2f} nodes {res.get('nodes', 0):5d} | rootLP {r.get('rootlp', 0):4.2f}s cuts~{r.get('cutdone') or 0:5.2f} heur {r.get('heurdone', 0):5.2f} (inc {r.get('rootinc')}) tree {r.get('tree', 0):5.2f} restarts {r['restarts']} | first {first[0] if first else '-'} | -309 at {f'{best[0][0]:.2f}s [{best[0][2]}]' if best else '-'}")
    if V:
        for t, z, s in r["inc"]:
            print(f"     inc {t:6.2f}s {z:9.2f} [{s}]")
        for s in r["subs"]:
            print("     " + s[:170])

"""logs/ のログを集計する。python summ.py [問題,...] [設定,...]
列: 時間 (時間切れは T)、再スタート数、根のヒューリスティクスの終わりの時刻 (再スタートごと)、その時の暫定解、木の開始時刻、
根のサブ MIP の時間の和 (parallel + 直列)、その本数、最終の下界。"""
import json, re, sys
from collections import defaultdict
from pathlib import Path
L = Path(__file__).resolve().parent / "logs"
probs = sys.argv[1].split(",") if len(sys.argv) > 1 and sys.argv[1] else None
cfgs = sys.argv[2].split(",") if len(sys.argv) > 2 else None
rows = defaultdict(list)
for f in sorted(L.glob("*.log")):
    m = re.match(r"(.+)_(.+)_(\d+)\.log$", f.name)
    p, c, r = m.group(1), m.group(2), int(m.group(3))
    if (probs and p not in probs) or (cfgs and c not in cfgs):
        continue
    t = f.read_text()
    res = None
    for line in t.splitlines()[::-1]:
        if line.startswith("{"):
            res = json.loads(line); break
    if res is None:
        rows[(p, c)].append((r, "ERR", "", "", "", "", "", "")); continue
    tm = f"{res['time']:.1f}" if res["status"] == "optimal" else ("T" if res["status"] == "time_limit" else res["status"])
    restarts = len(re.findall(r"^MIP: restart \d+ \(", t, re.M))
    ah = re.findall(r"after root heuristics: incumbent Some\(([\d.e+-]+)\) \(([\d.]+)s\)", t)
    tree = re.findall(r"tree search with \d+ threads \(([\d.]+)s\)", t)
    # 根のサブ MIP (木の開始より前) の時間: 並列 (parallel) と直列 (sub-MIP:) に分ける
    pre = t.split("tree search with")[0]
    par = [float(x) for x in re.findall(r"parallel sub-MIP \([^)]*\): status \w+, nodes \d+, objective [^,]+, ([\d.]+)s", pre)]
    ser = [float(x) for x in re.findall(r"sub-MIP: \d+ free columns of \d+, status \w+, nodes \d+, objective [^,]+, ([\d.]+)s", pre)]
    sub = par + ser
    rows[(p, c)].append((r, tm, restarts, "+".join(f"{float(b):.1f}" for a, b in ah), "/".join(f"{float(a):.0f}" for a, b in ah),
                         "/".join(tree), f"{sum(par):.1f}+{sum(ser):.1f}", len(sub), f"{res.get('bound', float('nan')):.0f}"))
print("問題 設定 回: 時間 再スタート 根ヒュー終わり 暫定解 木の開始 サブMIP秒和(並列+直列) 本数 下界")
for (p, c), v in sorted(rows.items()):
    for x in sorted(v):
        print(p, c, *x)

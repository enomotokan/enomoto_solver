"""logs/*.log から根ごとの経過を拾う: python diag.py [LOG ...]
根ごと (再スタートの回) に、カットのループの前後の LP 値・ラウンド数、早めの被約費用の記録での新たな固定、
再スタートの判定 (新たな固定 / 閾値)、再スタート後の列数 (RESTART_DROP_FIXED) を出す。最初の暫定解の時刻と値も。"""
import re, sys, glob
from pathlib import Path


def parse(path):
    roots, cur = [], None
    first_inc = None
    tree_lb = None
    for l in open(path, errors="replace"):
        if l.startswith("CUTMIX types restarts"):
            cur = {"r": int(l.split()[3]), "rounds": 0, "lp0": None, "lp1": None, "early": None, "check": None, "drop": None, "inc_at_cut": None}
            roots.append(cur)
        m = re.search(r"cut loop done: .* obj ([-\d.e+]+) -> ([-\d.e+]+)", l)
        if m and cur is not None and cur["lp1"] is None:
            cur["lp0"], cur["lp1"] = float(m[1]), float(m[2])
        if cur is not None and re.search(r"MIP: cut round \d+: \d+ cuts \(of", l):
            cur["rounds"] += 1
        m = re.search(r"early root reduced costs stored \((\d+) newly fixed", l)
        if m and cur is not None and cur["early"] is None: cur["early"] = int(m[1])
        m = re.search(r"restart check: newly fixed (\d+), integer columns (\d+), free at the root (\d+), threshold ([\d.]+)", l)
        if m and cur is not None: cur["check"] = (int(m[1]), int(m[2]), int(m[3]), float(m[4]))
        m = re.search(r"RESTART_DROP_FIXED: columns (\d+) -> (\d+)", l)
        if m and cur is not None: cur["drop"] = (int(m[1]), int(m[2]))
        m = re.search(r"TRSTAT: tree start .* lb ([-\d.e+]+) ub (\S+)", l)
        if m: tree_lb = (float(m[1]), m[2])
        if first_inc is None:
            m = re.search(r"^MIP: (?:root heuristic )?(.*?): (?:found|improved|new incumbent) .*?([-\d.]+e[-+]?\d+|[-\d.]+) \(([\d.]+)s\)", l)
            if m: first_inc = (m[1], m[3])
    return roots, tree_lb


def show(path):
    roots, tree_lb = parse(path)
    print(f"== {Path(path).name}  tree start lb/ub {tree_lb}")
    for c in roots:
        print(f"  root {c['r']}: cut rounds {c['rounds']}, LP {c['lp0']} -> {c['lp1']}, early fixed {c['early']}, restart check {c['check']}, dropped cols {c['drop']}")


if __name__ == "__main__":
    for f in sys.argv[1:] or sorted(glob.glob(str(Path(__file__).parent / "logs/*.log"))): show(f)

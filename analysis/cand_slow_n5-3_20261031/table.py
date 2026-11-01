"""logs/ の時間を問題 × 設定の表にする (各回の秒を / でつなぐ。時間切れは T)。python table.py [問題,...] [設定,...]"""
import json, re, sys
from collections import defaultdict
from pathlib import Path
L = Path(__file__).resolve().parent / "logs"
probs = sys.argv[1].split(",") if len(sys.argv) > 1 and sys.argv[1] else None
cfgs = sys.argv[2].split(",") if len(sys.argv) > 2 else None
t = defaultdict(dict)
for f in L.glob("*.log"):
    m = re.match(r"(.+)_(.+)_(\d+)\.log$", f.name)
    p, c, r = m.group(1), m.group(2), int(m.group(3))
    res = next((json.loads(l) for l in f.read_text().splitlines()[::-1] if l.startswith("{")), None)
    t[(p, c)][r] = "ERR" if res is None else (f"{res['time']:.1f}" if res["status"] == "optimal" else ("T" if res["status"] == "time_limit" else res["status"]))
P = probs or sorted({p for p, _ in t})
C = cfgs or sorted({c for _, c in t})
print("| 問題 | " + " | ".join(C) + " |")
print("|---" * (len(C) + 1) + "|")
for p in P:
    print(f"| {p} | " + " | ".join(" / ".join(v for _, v in sorted(t[(p, c)].items())) or "" for c in C) + " |")

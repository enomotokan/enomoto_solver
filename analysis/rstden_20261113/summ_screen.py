"""絞り込みの集計 (screen.jsonl): 問題ごとに、設定ごとの各種の秒 (T = 時間切れ) と再スタートの固定の数 [..]、RSTCRIT n で見送った数 (nN)。
使い方: python summ_screen.py [screen.jsonl]"""
import json, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
f = sys.argv[1] if len(sys.argv) > 1 else "screen.jsonl"
R = defaultdict(dict)
for l in (D / f).read_text().splitlines():
    if l.strip():
        r = json.loads(l); R[r["problem"]][r["cfg"], r["rep"]] = r
CF = ["c", "c12p", "c2", "c1", "c12pf"]
def cell(r):
    res = r["result"]; st = res.get("status")
    t = f'{res["time"]:.1f}' if st == "optimal" else ("T" if st in ("time_limit", "timelimit", "feasible", "time limit") else str(st))
    rs = [x[1] for x in r["restarts"]]
    sk = sum(1 for d in r["diag"] if d.get("tag") == "skipn")
    return f'{t} {rs}' + (f' n{sk}' if sk else "")
for p, d in R.items():
    print(p)
    for c in CF:
        cs = [cell(d[c, s]) for s in sorted({s for (cc, s) in d if cc == c})]
        if cs:
            print(f"  {c:6s} " + " | ".join(cs))

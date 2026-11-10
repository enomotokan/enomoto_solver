"""PLSB の計測の集計: 問題ごとの時間・ノード・強分岐の LP の数と反復・LP の反復 (最後の進みの行)・PLSB の判定の数。
使い方: python summ.py OUT.jsonl [基準の設定]"""
import json, re, sys, math
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
OUT = Path(sys.argv[1]); base = sys.argv[2] if len(sys.argv) > 2 else "def"
rows = [json.loads(l) for l in OUT.read_text().splitlines() if l.strip()]
TL = None
def stats(r):
    log = D / "logs" / f"{OUT.stem}_{r['problem']}_{r['cfg']}_s{r['rep']}.log"
    d = {"sb_lps": None, "sb_iters": None, "lp_iters": None, "stop": 0, "past": 0, "early": 0}
    if log.exists():
        for l in log.read_text().splitlines():
            m = re.search(r"lp_iters (\d+) sb_iters (\d+) sb_lps (\d+)", l)
            if m:
                d["lp_iters"], d["sb_iters"], d["sb_lps"] = map(int, m.groups())
            m = re.search(r"continued past lookahead (\d+), stopped before lookahead (\d+)", l)
            if m:
                d["past"] += int(m.group(1)); d["early"] += int(m.group(2))
    return d
tab = defaultdict(dict)
for r in rows:
    res = r["result"]; s = stats(r)
    tab[(r["problem"], r["rep"])][r["cfg"]] = (res, s)
cfgs = sorted({r["cfg"] for r in rows}, key=lambda c: (c != base, c))
def tt(res, tl):
    return res.get("time", tl) if res.get("status") == "optimal" else tl
tl = float(sys.argv[3]) if len(sys.argv) > 3 else 60.0
print("問題 種 | " + " | ".join(f"{c}: 時間 ノード sbLP sb反復 LP反復 早止/延長" for c in cfgs))
for k in sorted(tab):
    cells = []
    for c in cfgs:
        if c not in tab[k]:
            cells.append("-"); continue
        res, s = tab[k][c]
        gap = ""
        if res.get("status") != "optimal" and res.get("objective") is not None and res.get("bound") is not None:
            o, b = res["objective"], res["bound"]
            gap = f"(gap {abs(o-b)/max(abs(o),1e-9)*100:.2f}%)"
        cells.append(f"{tt(res,tl):.1f}{gap} {res.get('nodes')} {s['sb_lps']} {s['sb_iters']} {s['lp_iters']} {s['early']}/{s['past']}")
    print(f"{k[0]} {k[1]} | " + " | ".join(cells))
print()
for c in cfgs:
    ts = [tt(tab[k][c][0], tl) for k in tab if c in tab[k] and base in tab[k]]
    tb = [tt(tab[k][base][0], tl) for k in tab if c in tab[k] and base in tab[k]]
    sg = lambda xs: math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
    ns = sum(tab[k][c][0].get("status") == "optimal" and tt(tab[k][c][0], tl) < tl for k in tab if c in tab[k])
    print(f"{c}: SGM {sg(ts):.2f} (基準 {sg(tb):.2f}), 解けた {ns} / {sum(c in tab[k] for k in tab)}")

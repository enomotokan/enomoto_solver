"""根の比べ (root58.jsonl・rootmitt.jsonl) の表。使い方: python summ_root.py [jsonl ...]
- 根の下界: ENOMOTO はノード上限 1 の解法の bound (根のカットのループと根の再スタートの後)、HiGHS も mip_max_nodes=1 の mip_dual_bound。
- gap 閉じ (%) = (mk - def) / |best - def|、best は 3 つ (def・mk・HiGHS) と暫定解の値のうちの最良の主の値 (最小化)。
- mk の分離: 呼ばれた回数・候補の数・時間の和。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
files = sys.argv[1:] or ["root58.jsonl", "rootmitt.jsonl"]
for fn in files:
    if not (D / fn).exists():
        continue
    R = {}
    for l in open(D / fn):
        r = json.loads(l); R.setdefault(r["problem"], {})[r["cfg"]] = r
    print(f"== {fn} ({len(R)} 問)")
    print(f"{'problem':28s} {'def':>14s} {'mk':>14s} {'HiGHS':>14s} {'best primal':>14s} {'mk-def':>10s} {'閉じ%':>7s} {'H>def':>5s} | mk: 回 候補 秒 / def秒 mk秒")
    rows = []
    for p, d in R.items():
        def g(c, k="bound"):
            x = d.get(c)
            v = None if not x else x["result"].get(k)
            return v if isinstance(v, (int, float)) else None
        bd, bm, bh = g("def"), g("mk"), g("highs")
        prim = [v for v in (g("def", "objective"), g("mk", "objective"), g("highs", "objective")) if v is not None]
        best = min(prim) if prim else None
        diff = None if bd is None or bm is None else bm - bd
        sc = 1e-6 * max(1.0, abs(bd or 0))
        clo = None
        if diff is not None and best is not None and best - bd > sc:
            clo = 100 * diff / (best - bd)
        hgt = bh is not None and bd is not None and bh - bd > 1e-4 * max(1, abs(bd))
        s = d.get("mk", {}).get("sep", {})
        rows.append((p, bd, bm, bh, best, diff, clo, hgt, s, g("def", "time"), g("mk", "time")))
    def f(v, w=14):
        return f"{v:{w}.6g}" if isinstance(v, (int, float)) else f"{'-':>{w}s}"
    for p, bd, bm, bh, best, diff, clo, hgt, s, td, tm in rows:
        mark = "*" if diff is not None and abs(diff) > 1e-6 * max(1, abs(bd)) else " "
        print(f"{p:27s}{mark} {f(bd)} {f(bm)} {f(bh)} {f(best)} {f(diff,10)} {f(clo,7)} {'H' if hgt else '':>5s} | {s.get('calls','-')} {s.get('cands','-')} {s.get('time',0):.2f} / {f(td,6)} {f(tm,6)}")
    up = [r for r in rows if r[5] is not None and r[5] > 1e-6 * max(1, abs(r[1]))]
    dn = [r for r in rows if r[5] is not None and r[5] < -1e-6 * max(1, abs(r[1]))]
    print(f"mk が根の下界を上げた {len(up)} 問: {[r[0] for r in up]}")
    print(f"mk が根の下界を下げた {len(dn)} 問: {[r[0] for r in dn]}")
    print(f"HiGHS が def より上: {sum(r[7] for r in rows)} 問、そのうち mk が上げた {sum(r[7] and r in up for r in rows)} 問")

"""ROOTSB の全部を解く比べ (python rsbfull.py res_RSB_full_P.jsonl [res_RSB_full_M.jsonl])。1 行 = (問題, 種)、def と ilf の
状態・時間・ノード・下界 (時間切れのとき)。SGM は shift 1 秒、時間切れは上限で数える。"""
import json, sys, math
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
for fn in sys.argv[1:]:
    R = defaultdict(dict)
    order = []
    for l in open(D / fn):
        r = json.loads(l)
        k = (r["problem"], r["seed"])
        if k not in order:
            order.append(k)
        R[k][r.get("cfg", "def")] = (r["result"], r["tl"])
    print(f"== {fn}")
    print(f"{'problem':22s} sd | {'def':>10s} {'秒':>6s} {'nodes':>8s} {'bound':>12s} | {'ilf':>10s} {'秒':>6s} {'nodes':>8s} {'bound':>12s}")
    lg = []
    for k in order:
        d = R[k]
        cells = []
        for c in ("def", "ilf"):
            if c in d:
                x, tl = d[c]
                cells.append(f"{str(x.get('status'))[:10]:>10s} {x.get('time') or 0:6.1f} {x.get('nodes') or 0:8d} {x.get('bound') if x.get('bound') is not None else float('nan'):12.6g}")
            else:
                cells.append(f"{'-':>10s} {'':6s} {'':8s} {'':12s}")
        print(f"{k[0]:22s} {k[1]:2d} | " + " | ".join(cells))
        if "def" in d and "ilf" in d:
            t = [min(d[c][0].get("time") or d[c][1], d[c][1]) if d[c][0].get("status") == "optimal" else d[c][1] for c in ("def", "ilf")]
            lg.append(math.log((t[1] + 1) / (t[0] + 1)))
    if lg:
        print(f"そろった {len(lg)} 回: SGM の比 ilf/def = {math.exp(sum(lg) / len(lg)):.3f}、ilf が速い {sum(x < -0.05 for x in lg)}・遅い {sum(x > 0.05 for x in lg)} (5% で)")

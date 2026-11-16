"""まとめ: python summ.py > summ.txt。問題ごとに、各設定の full との時間の比 (60 秒の問題は種 1〜4 の幾何平均、shift 1 秒、時間切れは制限時間)
と、tbfp-network は種ごとの時間 (T = 時間切れ)。"""
import json, glob, math, sys
from pathlib import Path
D = Path(__file__).resolve().parent; sys.path.insert(0, str(D))
from cfg import CFGS, P60, P600  # noqa: E402
rows = [json.loads(l) for f in sorted(glob.glob(str(D / "res_s*.jsonl"))) for l in open(f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
ok = lambda r: r["result"].get("status") in ("optimal", "infeasible")
t = lambda r: min(r["result"]["time"], r["tl"]) if ok(r) else r["tl"]
print(f"行数 {len(rows)}、crash {[k for k, r in R.items() if r['result'].get('status') in (None, 'crash')]}")
print("食い違い:", [(p, s) for p in P60 + P600 for s in (1, 2, 3, 4) if len({round(R[p, c, s]['result']['objective'], 4) for c in CFGS if (p, c, s) in R and ok(R[p, c, s]) and R[p, c, s]['result'].get('objective') is not None}) > 1])
print("\n設定 | " + " | ".join(P60) + " | 3 問の幾何平均 (full との比、1 未満なら外すと速い) | " + " | ".join(f"{p} 種1・2 (秒)" for p in P600))
for c in CFGS:
    cells, logs = [], []
    for p in P60:
        v = [math.log((t(R[p, c, s]) + 1) / (t(R[p, "full", s]) + 1)) for s in (1, 2, 3, 4) if (p, c, s) in R and (p, "full", s) in R]
        cells.append(f"{math.exp(sum(v) / len(v)):.2f}" if v else "-"); logs += v
    tb = [(f"{t(R[p, c, s]):.0f}" if ok(R[p, c, s]) else "T") if (p, c, s) in R else "-" for p in P600 for s in (1, 2)]
    print(f"{c} | " + " | ".join(cells) + f" | {math.exp(sum(logs) / len(logs)) if logs else float('nan'):.2f} | " + " ・ ".join(tb))

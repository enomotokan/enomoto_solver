"""まとめ: python summ.py > summ.txt。問題 × 種ごとの各設定の秒 (T = 時間切れ) と目的値・下界。"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent; sys.path.insert(0, str(D))
from cfg import CFGS, PROBS  # noqa: E402
R = {(r["problem"], r["cfg"], r["seed"]): r for f in sorted(glob.glob(str(D / "res_s*.jsonl"))) for l in open(f) if l.strip() for r in [json.loads(l)]}
ok = lambda r: r["result"].get("status") in ("optimal", "infeasible")
print("問題 種 | " + " | ".join(CFGS))
for p in PROBS:
    for s in (1, 2):
        cells = []
        for c in CFGS:
            r = R.get((p, c, s))
            cells.append("-" if r is None else f"{r['result']['time']:.0f}" if ok(r) else f"T ({r['result'].get('objective')}, {r['result'].get('bound')})")
        print(f"{p} {s} | " + " | ".join(cells))
for c in CFGS:
    print(c, "解けた", sum(ok(r) for (p, cc, s), r in R.items() if cc == c), "/", sum(1 for k in R if k[1] == c))

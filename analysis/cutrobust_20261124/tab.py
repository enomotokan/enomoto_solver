"""res.jsonl の集計: python tab.py [問題]  名札の接頭辞 (tf・fb・mp・q・g、w_def・w_q・w_g) ごとに、出発の頂点ごとの
(根のラウンド数, 根の下界, ノード, 時間) を並べる。時間 ≥ 59.9 は時間切れ。"""
import json, sys, collections
from pathlib import Path
D = Path(__file__).resolve().parent
want = sys.argv[1] if len(sys.argv) > 1 else None
rows = [json.loads(l) for l in open(D / "res.jsonl")]
tab = collections.OrderedDict()
for r in rows:
    if want and r["problem"] != want: continue
    if "_start" not in r["tag"] or "result" not in r: continue
    cfg, st = r["tag"].rsplit("_start", 1)
    if not st.isdigit(): continue
    res = r["result"]; t = float(res[5]); rb = float(res[4]) if res[4] not in ("None",) else float("nan")
    tab.setdefault((r["problem"], cfg), {})[int(st)] = (len(r["rounds"]), rb, int(res[3]), t, res[0])
for (p, cfg), d in tab.items():
    cells = [f"s{s}: {v[0]}r {v[1]:.6g} n{v[2]} {v[3]:.1f}s{'' if v[4]=='optimal' else ' ' + v[4]}" for s, v in sorted(d.items())]
    ts = [v[3] for v in d.values()]
    print(f"{p} {cfg}: 時間 中央値 {sorted(ts)[len(ts)//2]:.1f} 最大 {max(ts):.1f}, ノード>0 {sum(v[2]>0 for v in d.values())}/{len(d)} | " + "; ".join(cells))

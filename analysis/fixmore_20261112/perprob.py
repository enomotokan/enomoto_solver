"""問題ごとの def と u の比べ (analysis/fixmore_20261112.md): 時間 (解けなければ 60)、再スタートの回数、u で内点法の双対が足した固定の数。
使い方: python perprob.py [res の glob]"""
import json, glob, math, sys, collections
from pathlib import Path
D = Path(__file__).resolve().parent
R = collections.defaultdict(dict)
for f in sorted(glob.glob(str(D / (sys.argv[1] if len(sys.argv) > 1 else "res*_s?.jsonl")))):
    for l in open(f):
        r = json.loads(l); res = r["result"]
        t = res.get("time") if res.get("status") == "optimal" else 60.0
        R[r["problem"]][(r["cfg"], r["rep"])] = (t, len(r.get("restarts", [])), sum(x[2] - x[1] for x in (r.get("fixmore") or [])), len(r.get("fixmore") or []))
rows = []
for p, v in R.items():
    reps = sorted({k[1] for k in v})
    lr = [math.log((v[("u", s)][0] + 10) / (v[("def", s)][0] + 10)) for s in reps if ("u", s) in v and ("def", s) in v]
    rows.append((sum(lr) / len(lr), p, reps, v))
for m, p, reps, v in sorted(rows):
    d = " ".join(f"{v[('def', s)][0]:.1f}/{v[('u', s)][0]:.1f}" for s in reps)
    rs = " ".join(f"{v[('def', s)][1]}/{v[('u', s)][1]}" for s in reps)
    fx = " ".join(f"{v[('u', s)][2]}({v[('u', s)][3]})" for s in reps)
    print(f"{p:28s} {100 * (math.exp(m) - 1):+6.1f}%  時間 def/u {d:32s} 再スタート {rs:14s} 足した固定(回) {fx}")

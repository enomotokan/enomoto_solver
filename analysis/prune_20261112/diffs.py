"""設定ごとに、def と解けたかどうかが違う (問題, 種) を並べる。使い方: python diffs.py [設定 ...]"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for f in sorted(glob.glob(str(D / "res_P_s*.jsonl"))) for l in open(f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
ok = lambda r: r["result"].get("status") in ("optimal", "infeasible")
cfgs = sys.argv[1:] or sorted({k[1] for k in R} - {"def"})
for c in cfgs:
    lose = [(p, s, round(R[(p, "def", s)]["result"]["time"], 1)) for (p, cc, s) in R if cc == c and ok(R[(p, "def", s)]) and not ok(R[(p, c, s)])]
    gain = [(p, s, round(R[(p, c, s)]["result"]["time"], 1)) for (p, cc, s) in R if cc == c and ok(R[(p, c, s)]) and not ok(R[(p, "def", s)])]
    print(f"{c}: 失う {sorted(lose)} / 得る {sorted(gain)}")

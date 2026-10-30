"""bench.jsonl の集計: 設定ごとの SGM (ずらし 10 秒、解けなければ 60 秒)・解けた数 (負けている 21 問 / MIPLIB 40 問 / 全 58 問)、
最適値の食い違い、問題ごとの時間 (差の大きいものに印)。"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parents[0] / "cbcimpl_20261029"))
from probs import LOSS, PROBS, SETS  # noqa: E402
R = defaultdict(dict)
for l in open(here / "bench.jsonl"):
    r = json.loads(l); R[(r["problem"], r["rep"])][r["cfg"]] = r["result"]
cfgs = sys.argv[1].split(",") if len(sys.argv) > 1 else ["def", "hm3_cfl", "h1", "hm3_cfl_h1"]
def t(res): return res.get("time", 60) if res.get("status") == "optimal" else 60.0
reps = sorted({k[1] for k in R})
sets = {"loss21": LOSS, "miplib40": SETS["miplib40"], "all58": PROBS}
for rep in reps:
    common = [p for p in PROBS if all(c in R[(p, rep)] for c in cfgs)]
    print(f"rep {rep}: 全設定が揃った問題 {len(common)}")
    for sn, ps in sets.items():
        ps = [p for p in ps if p in common]
        row = []
        for c in cfgs:
            xs = [t(R[(p, rep)][c]) for p in ps]
            sgm = math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10 if xs else float("nan")
            row.append(f"{c}: {sgm:6.2f}s ({sum(R[(p, rep)][c].get('status') == 'optimal' for p in ps)}/{len(ps)})")
        print(f"  {sn:9s} " + "   ".join(row))
print("\n食い違い・異常:")
for (p, rep), d in sorted(R.items()):
    opts = {c: v["objective"] for c, v in d.items() if v.get("status") == "optimal"}
    if opts and max(opts.values()) - min(opts.values()) > 1e-6 * max(1, abs(max(opts.values()))):
        print(" ", p, rep, opts)
    for c, v in d.items():
        if v.get("status") not in ("optimal", "time_limit"):
            print("  status", p, rep, c, v.get("status"), str(v.get("error", ""))[:100])
print("\n問題ごと (時間。TL は時間切れ、括弧は暫定解):")
for p in PROBS:
    for rep in reps:
        d = R[(p, rep)]
        if not d:
            continue
        cell = [(f"{d[c]['time']:5.1f}" if d[c].get("status") == "optimal" else f"TL({d[c].get('objective')})") if c in d else "-" for c in cfgs]
        ts = [t(d[c]) for c in cfgs if c in d]
        flag = " *" if ts and max(ts) > 1.3 * min(ts) + 3 else ""
        print(f"  {p:24s} r{rep} " + " / ".join(cell) + flag)

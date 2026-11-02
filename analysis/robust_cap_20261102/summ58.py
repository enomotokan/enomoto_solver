"""bench.jsonl の集計 (analysis/halve_submip_20261030/summ.py をもとに): 設定ごとの SGM (ずらし 10 秒、解けなければ 60 秒)・解けた数、
問題ごとの時間 (片方だけ解けた・差が 1.5 倍以上のもの)、試作 p が働いた回数 (logs58 の表示)。使い方: python summ58.py [設定,...]"""
import json, math, re, sys
from collections import defaultdict
from pathlib import Path
here = Path(__file__).resolve().parent
sys.path.insert(0, str(here.parents[0] / "cbcimpl_20261029"))
from probs import LOSS, PROBS, SETS  # noqa: E402
R = defaultdict(dict)
for l in open(here / "bench.jsonl"):
    r = json.loads(l); R[(r["problem"], r["rep"])][r["cfg"]] = r["result"]
cfgs = sys.argv[1].split(",") if len(sys.argv) > 1 else ["def", "p"]
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
print("\n時間の上限 (60 秒) を 1 秒以上超えた回:")
for (p, rep), d in sorted(R.items()):
    for c, v in d.items():
        if v.get("time") and v["time"] > 61:
            print(f"  {p} {c} {rep}: {v['time']:.1f}s")
        if v.get("status") not in ("optimal", "time_limit"):
            print("  status", p, rep, c, v.get("status"), str(v.get("error", ""))[:100])
print("\n問題ごとの違い (片方だけ解けた・1.5 倍以上の差) と p の働いた回数 (進みで打ち切り / 上限を超えて解けた):")
for p in PROBS:
    for rep in reps:
        d = R[(p, rep)]
        if not all(c in d for c in cfgs):
            continue
        ts = [t(d[c]) for c in cfgs]
        log = here / "logs58" / f"{p}_p_{rep}.log"
        txt = log.read_text(errors="replace") if log.exists() else ""
        np_, nx = txt.count("stopped by the progress check"), txt.count("went past the cap")
        if max(ts) / max(min(ts), 1e-9) >= 1.5 and max(ts) >= 5 or np_ or nx:
            print(f"  {p} {rep}: " + "  ".join(f"{c} {'TL' if d[c].get('status') != 'optimal' else round(d[c]['time'], 1)}" for c in cfgs) + (f"   p: 打ち切り {np_}・超えて解けた {nx}" if np_ or nx else ""))

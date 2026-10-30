"""jsonl の集計: 設定 x 回 x 組 (負けている 21 問・MIPLIB 40 問) の SGM (ずらし 10 秒、解けなければ 60 秒)・解けた数、
問題ごとの大きな差、最適値の食い違い。使い方: python summ.py <jsonl> [<jsonl> ...]"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from probs import SETS  # noqa: E402
R = defaultdict(dict)
cfgs = []
for fn in sys.argv[1:]:
    for l in open(fn, encoding="utf-8"):
        r = json.loads(l)
        R[(r["problem"], r["rep"])][r["cfg"]] = r["result"]
        if r["cfg"] not in cfgs:
            cfgs.append(r["cfg"])
def t(res): return min(res.get("time", 60), 60.0) if res.get("status") == "optimal" else 60.0
reps = sorted({k[1] for k in R})
def sgm(xs): return math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
for sn, ps in SETS.items():
    print(f"== {sn}")
    for c in cfgs:
        row = []
        allx = []
        for rep in reps:
            xs = [t(R[(p, rep)][c]) for p in ps if c in R[(p, rep)]]
            if xs:
                row.append(f"{sgm(xs):6.2f}s ({sum(x < 60 for x in xs)}/{len(xs)})")
                allx += xs
        print(f"  {c:14s} " + "  ".join(row))
print("\n問題ごと (各回の時間、base と 20% 以上かつ 2 秒以上違うもの):")
for (p, rep), d in sorted(R.items()):
    if "base" not in d:
        continue
    b = t(d["base"])
    diffs = [f"{c}={t(v):.1f}" for c, v in d.items() if c != "base" and abs(t(v) - b) > max(2.0, 0.2 * b)]
    if diffs:
        print(f"  {p:22s} rep{rep} base={b:.1f}  " + " ".join(diffs))
print("\n食い違い (どれかが optimal で目的値が違う) と異常終了:")
for (p, rep), d in sorted(R.items()):
    opts = {c: v["objective"] for c, v in d.items() if v.get("status") == "optimal"}
    if opts and max(opts.values()) - min(opts.values()) > 1e-6 * max(1, abs(max(opts.values()))):
        print(f"  {p} rep{rep}: {opts}")
    for c, v in d.items():
        if v.get("status") in ("crash", "error"):
            print(f"  {p} rep{rep} {c}: {v}")

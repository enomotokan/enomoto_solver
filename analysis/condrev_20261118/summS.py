"""短い計測 S の集計: python summS.py > summS.txt
試作ごとに、問題 × 種の時間 (秒、時間切れは T) と、def に対する対の SGM の比 (shift 1 秒、時間切れは上限で数える、
どれかの設定が解けた問題だけ。区間は問題単位のブートストラップ 95%) を出す。600 秒の問題は名前に @600 を付ける。"""
import json, glob, math, random
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
R = {}
for f in sorted(glob.glob(str(D / "res_S_s*.jsonl"))):
    for l in open(f):
        if l.strip():
            r = json.loads(l)
            # mzzv42z は 60 秒 (sbt) と 600 秒 (SUBFIX) の両方にあるので、600 秒の問題は名前に印を付けて分ける
            R[(r["problem"] + ("@600" if r["tl"] == 600 else ""), r["cfg"], r["seed"])] = r
def ok(r): return str(r["result"].get("status", "")).lower() in ("optimal", "infeasible")
def t(r): return min(r["result"].get("time", r["tl"]), r["tl"]) if ok(r) else r["tl"]
def cell(r):
    if r is None: return "-"
    if r["result"].get("status") == "crash": return "X"
    return f"{t(r):.1f}" if ok(r) else "T"
GROUPS = {
    "sbt (一般整数 16 問 + neos-911970・10teams)": ["sbt", "sbtc"],
    "SUBFIX": ["sf0", "sfcap"],
    "rb": ["rb"],
}
probs = sorted({k[0] for k in R})
seeds = sorted({k[2] for k in R})
for g, alts in GROUPS.items():
    ps = [p for p in probs if any((p, a, s) in R for a in alts for s in seeds)]
    print(f"## {g}\n")
    print("| 問題 | " + " | ".join(["def"] + alts) + " |")
    print("|---" * (len(alts) + 2) + "|")
    for p in ps:
        row = [p] + [" / ".join(cell(R.get((p, c, s))) for s in seeds if any((p, x, s) in R for x in ["def"] + alts)) for c in ["def"] + alts]
        print("| " + " | ".join(row) + " |")
    print()
    for a in alts:
        lr = defaultdict(list); sd = sa = 0; n = 0
        for p in ps:
            for s in seeds:
                d, x = R.get((p, "def", s)), R.get((p, a, s))
                if d is None or x is None: continue
                n += 1; sd += ok(d); sa += ok(x)
                if not (ok(d) or ok(x)): continue
                lr[p].append(math.log((t(x) + 1) / (t(d) + 1)))
        if not lr: continue
        pm = {p: sum(v) / len(v) for p, v in lr.items()}
        allv = [v for vs in lr.values() for v in vs]
        m = sum(allv) / len(allv)
        rng = random.Random(0); bs = []
        keys = list(lr)
        for _ in range(2000):
            smp = [v for k in (rng.choice(keys) for _ in keys) for v in lr[k]]
            bs.append(sum(smp) / len(smp))
        bs.sort()
        bad = [p for p in keys if len(lr[p]) >= 2 and all(v > math.log(1.25) for v in lr[p])]
        good = [p for p in keys if len(lr[p]) >= 2 and all(v < math.log(0.8) for v in lr[p])]
        print(f"- {a}: 対 {n} 回、SGM 比 {math.exp(m) - 1:+.1%} [{math.exp(bs[50]) - 1:+.1%}, {math.exp(bs[1949]) - 1:+.1%}]、解けた {sd}→{sa}、"
              f"揃って 1.25 倍以上遅い: {'・'.join(bad) or 'なし'}、揃って 0.8 倍以下: {'・'.join(good) or 'なし'}")
    print()

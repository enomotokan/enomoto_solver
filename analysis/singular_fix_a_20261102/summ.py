"""run.py の jsonl の集計 (Markdown を標準出力に)。時間切れ・異常終了は 60 秒。反復は NB_DIAG の全場所の it_mis + it_ok の和
(main と sub のスレッドの和)。特異で捨てた LP (n_noeff) と差し替えた LP (n_rep) は main + sub の全場所の和。

使い方: python summ.py <jsonl> [--sgm]
"""
import json
import math
import sys
from collections import defaultdict

fn = sys.argv[1]
R = defaultdict(dict)
for l in open(fn, encoding="utf-8"):
    r = json.loads(l)
    R[(r["problem"], r["rep"])][r["cfg"]] = r


def t(r):
    x = r["result"]
    return min(x.get("time", 60), 60.0) if x.get("status") == "optimal" else 60.0


def diag(r):
    out = defaultdict(float)
    site = defaultdict(lambda: defaultdict(float))
    for line in r.get("diag", []):
        w = line.split()
        if w[1] == "total":
            continue
        kv = {k: float(v) for k, v in (x.split("=") for x in w[3:])}
        for k, v in kv.items():
            out[k] += v
            site[f"{w[1]} {w[2]}"][k] += v
    return out, site


def sgm(xs, s=10):
    return math.exp(sum(math.log(x + s) for x in xs) / len(xs)) - s


cfgs = ["base"] + sorted({c for d in R.values() for c in d} - {"base"})
probs = []
for (p, _), d in R.items():
    if p not in probs:
        probs.append(p)
print("| 問題 | 回数 | " + " | ".join(f"時間 {c}" for c in cfgs) + " | " + " | ".join(f"反復 {c}" for c in cfgs) + " | "
      + " | ".join(f"特異で捨てた LP {c}" for c in cfgs) + " | " + " | ".join(f"差し替えた LP {c} (列の平均)" for c in cfgs[1:]) + " | 解けた |")
print("|---|---|" + "---|" * (4 * len(cfgs) - 1 + 1))
allk = []
for p in probs:
    ks = [k for k in R if k[0] == p and all(c in R[k] for c in cfgs)]
    if not ks:
        continue
    allk += ks
    row = {}
    for c in cfgs:
        ts = [t(R[k][c]) for k in ks]
        ds = [diag(R[k][c])[0] for k in ks]
        it = [d["it_mis"] + d["it_ok"] for d in ds]
        row[c] = dict(t=ts, it=sum(it) / len(it), noeff=sum(d["n_noeff"] for d in ds) / len(ds), rep=sum(d["n_rep"] for d in ds) / len(ds),
                      srep=sum(d["sum_rep"] for d in ds), nrep=sum(d["n_rep"] for d in ds),
                      ok=sum(R[k][c]["result"].get("status") == "optimal" for k in ks))
    fmt = lambda ts: " / ".join(f"{x:.1f}" for x in ts)
    print(f"| {p} | {len(ks)} | " + " | ".join(fmt(row[c]["t"]) for c in cfgs) + " | " + " | ".join(f"{row[c]['it']:.0f}" for c in cfgs) + " | "
          + " | ".join(f"{row[c]['noeff']:.0f}" for c in cfgs) + " | "
          + " | ".join(f"{row[c]['rep']:.0f} ({row[c]['srep'] / row[c]['nrep'] if row[c]['nrep'] else 0:.1f})" for c in cfgs[1:]) + " | "
          + "/".join(str(row[c]["ok"]) for c in cfgs) + " |")
if allk:
    sb = sgm([t(R[k]["base"]) for k in allk])
    print(f"\nSGM (shift 10 秒、{len(allk)} 組): " + "、".join(f"{c} {sgm([t(R[k][c]) for k in allk]):.2f} 秒 ({(sgm([t(R[k][c]) for k in allk]) / sb - 1) * 100:+.1f}%)" for c in cfgs))
    solved = {c: sum(R[k][c]["result"].get("status") == "optimal" for k in allk) for c in cfgs}
    print("解けた数: " + "、".join(f"{c} {v}" for c, v in solved.items()))
    for c in cfgs[1:]:
        gain = sorted({k[0] for k in allk if R[k][c]["result"].get("status") == "optimal" and R[k]["base"]["result"].get("status") != "optimal"})
        lost = sorted({k[0] for k in allk if R[k][c]["result"].get("status") != "optimal" and R[k]["base"]["result"].get("status") == "optimal"})
        print(f"{c}: 既定が解けず→解けた {gain}、既定が解けた→解けず {lost}")
    objs = defaultdict(set)
    for k in allk:
        for c in cfgs:
            x = R[k][c]["result"]
            if x.get("status") == "optimal":
                objs[k[0]].add(round(x["objective"], 4))
    print("最適値の食い違い:", [p for p, v in objs.items() if len(v) > 1])
    print("異常終了:", [(k, c, R[k][c]["result"].get("status")) for k in allk for c in cfgs if R[k][c]["result"].get("status") not in ("optimal", "time_limit")])
# 場所ごとの特異で捨てた LP・差し替えた LP (全回の和)
print("\n| 場所 | " + " | ".join(f"特異で捨てた LP {c} (反復 / 時間)" for c in cfgs) + " | " + " | ".join(f"差し替えた LP {c} (列の和、反復 / 時間)" for c in cfgs[1:]) + " |")
print("|---|" + "---|" * (2 * len(cfgs) - 1))
S = {c: defaultdict(lambda: defaultdict(float)) for c in cfgs}
for k in allk:
    for c in cfgs:
        for s_, kv in diag(R[k][c])[1].items():
            for kk, v in kv.items():
                S[c][s_][kk] += v
for s_ in sorted(set().union(*[set(S[c]) for c in cfgs])):
    if not any(S[c][s_]["n_noeff"] or S[c][s_]["n_rep"] for c in cfgs):
        continue
    print(f"| {s_} | " + " | ".join(f"{S[c][s_]['n_noeff']:.0f} ({S[c][s_]['it_noeff']:.0f} / {S[c][s_]['t_noeff']:.1f} 秒)" for c in cfgs) + " | "
          + " | ".join(f"{S[c][s_]['n_rep']:.0f} ({S[c][s_]['sum_rep']:.0f}、{S[c][s_]['it_rep']:.0f} / {S[c][s_]['t_rep']:.1f} 秒)" for c in cfgs[1:]) + " |")

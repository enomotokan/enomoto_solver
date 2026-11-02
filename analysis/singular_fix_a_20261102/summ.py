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


cfgs = ["base", "rep"]
probs = []
for (p, _), d in R.items():
    if p not in probs:
        probs.append(p)
print("| 問題 | 回数 | 時間 base | 時間 rep | 反復 base | 反復 rep | 特異で捨てた LP base | rep | 差し替えた LP rep (列の平均) | 解けた base/rep |")
print("|---|---|---|---|---|---|---|---|---|---|")
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
    b, q = row["base"], row["rep"]
    fmt = lambda ts: " / ".join(f"{x:.1f}" for x in ts)
    print(f"| {p} | {len(ks)} | {fmt(b['t'])} | {fmt(q['t'])} | {b['it']:.0f} | {q['it']:.0f} | {b['noeff']:.0f} | {q['noeff']:.0f} | "
          f"{q['rep']:.0f} ({q['srep'] / q['nrep'] if q['nrep'] else 0:.1f}) | {b['ok']}/{q['ok']} |")
if allk:
    sb, sr = sgm([t(R[k]["base"]) for k in allk]), sgm([t(R[k]["rep"]) for k in allk])
    print(f"\nSGM (shift 10 秒、{len(allk)} 組): base {sb:.2f} 秒、rep {sr:.2f} 秒 ({(sr / sb - 1) * 100:+.1f}%)")
    objs = defaultdict(set)
    for k in allk:
        for c in cfgs:
            x = R[k][c]["result"]
            if x.get("status") == "optimal":
                objs[k[0]].add(round(x["objective"], 4))
    print("最適値の食い違い:", [p for p, v in objs.items() if len(v) > 1])
    print("異常終了:", [(k, c, R[k][c]["result"].get("status")) for k in allk for c in cfgs if R[k][c]["result"].get("status") not in ("optimal", "time_limit")])
# 場所ごとの特異で捨てた LP・差し替えた LP (全回の和)
print("\n| 場所 | 特異で捨てた LP base (反復/時間) | 特異で捨てた LP rep | 差し替えた LP rep (列の和、反復/時間) |")
print("|---|---|---|---|")
S = {c: defaultdict(lambda: defaultdict(float)) for c in cfgs}
for k in allk:
    for c in cfgs:
        for s, kv in diag(R[k][c])[1].items():
            for kk, v in kv.items():
                S[c][s][kk] += v
for s in sorted(set(S["base"]) | set(S["rep"])):
    b, q = S["base"][s], S["rep"][s]
    if not (b["n_noeff"] or q["n_noeff"] or q["n_rep"]):
        continue
    print(f"| {s} | {b['n_noeff']:.0f} ({b['it_noeff']:.0f} / {b['t_noeff']:.1f} 秒) | {q['n_noeff']:.0f} ({q['it_noeff']:.0f} / {q['t_noeff']:.1f} 秒) | "
          f"{q['n_rep']:.0f} ({q['sum_rep']:.0f}、{q['it_rep']:.0f} / {q['t_rep']:.1f} 秒) |")

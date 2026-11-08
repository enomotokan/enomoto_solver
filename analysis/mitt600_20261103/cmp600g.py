"""Mittelmann 186 問の 600 秒の比べ res6・res7 の 2 x 2 の集計 (2026-11-08)。zerohalf のあり・なし × 既定・候補の形:
res6 の既定 ("ours")・cand1108 ("ours:cand1108") と res7 の nozh ("ours:nozh" = 既定 + zerohalf なし)・cand1108z ("ours:cand1108z")。
種 1・2 は同じ種 (`ENOMOTO_T_MIP_SEED`)。数え方は cmp600e.py と同じ。res7 の欠けた行は、その (問題, 種) を 4 つとも比べから外す。
使い方: python cmp600g.py"""
import json, math, glob
from pathlib import Path
D = Path(__file__).resolve().parent
TL = 600.0
K = [("既定", "res6", "ours"), ("cand1108", "res6", "ours:cand1108"), ("nozh", "res7", "ours:nozh"), ("cand1108z", "res7", "ours:cand1108z")]
R = {}  # R[(問題, 種)][名前]
for s in (1, 2):
    for tag in ("res6", "res7"):
        for f in sorted(glob.glob(str(D / f"{tag}s{s}_s*.jsonl"))):
            for l in open(f):
                r = json.loads(l)
                for name, t, sol in K:
                    if t == tag and r["solver"] == sol:
                        R.setdefault((r["problem"], s), {})[name] = r
H = {}
for f in sorted(glob.glob(str(D / "res_s*.jsonl"))):
    for l in open(f):
        r = json.loads(l)
        if r["solver"] == "highs":
            H[r["problem"]] = r

def fin(v): return v is not None and isinstance(v, (int, float)) and math.isfinite(v)
def norm(r):
    x = r["result"]; st = str(x.get("status", "")).lower(); t = x.get("time")
    solved = (st.startswith("optimal") or st.startswith("infeasible")) and t is not None and t <= TL + 5
    obj, bd = x.get("objective"), x.get("bound")
    gap = abs(obj - bd) / max(abs(obj), 1.0) if fin(obj) and fin(bd) else math.inf
    if solved: gap = 0.0
    return dict(solved=solved, t=min(t, TL) if solved else TL, st=st, obj=obj if fin(obj) else None, bd=bd if fin(bd) else None, gap=gap, crash=st == "crash")
def sgm(ts): return math.exp(sum(math.log(t + 10) for t in ts) / len(ts)) - 10
def desc(x): return f"{x['t']:.1f}" if x["solved"] else ("TL" + (f" gap {x['gap']:.1e}" if not math.isinf(x["gap"]) else " 解なし"))

keys = sorted(k for k, v in R.items() if len(v) == 4)
print(f"4 つそろった (問題, 種): {len(keys)} (種 1: {sum(1 for k in keys if k[1] == 1)}、種 2: {sum(1 for k in keys if k[1] == 2)})")
names = [n for n, _, _ in K]
for s in (1, 2, None):
    ks = [k for k in keys if s is None or k[1] == s]
    if not ks: continue
    lab = f"種 {s}" if s else "2 種"
    base = sgm([norm(R[k]["既定"])["t"] for k in ks])
    parts = []
    for n in names:
        xs = [norm(R[k][n]) for k in ks]
        ho = sum(1 for k in ks if k[0] in H and norm(H[k[0]])["solved"] and not norm(R[k][n])["solved"])
        parts.append(f"{n} 解けた {sum(x['solved'] for x in xs)} SGM {sgm([x['t'] for x in xs]):.2f} (比 {sgm([x['t'] for x in xs]) / base:.3f}) HiGHSだけ {ho}")
    print(f"{lab} ({len(ks)}): " + " | ".join(parts))
print("\n2 種とも解けたかが変わった (既定と比べて):")
probs = sorted({k[0] for k in keys})
for p in probs:
    ks = [k for k in keys if k[0] == p]
    if len(ks) < 2: continue
    for n in names[1:]:
        a = [norm(R[k]["既定"])["solved"] for k in ks]; b = [norm(R[k][n])["solved"] for k in ks]
        if all(x and not y for x, y in zip(a, b)): print(f"  {n:10s} だけ解けない: {p} (既定 " + "・".join(desc(norm(R[k]['既定'])) for k in ks) + ")")
        if all(y and not x for x, y in zip(a, b)): print(f"  {n:10s} だけ解ける:   {p} (" + "・".join(desc(norm(R[k][n])) for k in ks) + ")")
print("\nzerohalf を外した効き目が 2 種とも同じ向きに 1.5 倍以上 (既定 → nozh、cand1108 → cand1108z、両方解けた回):")
for p in probs:
    ks = [k for k in keys if k[0] == p]
    if len(ks) < 2: continue
    for a, b in (("既定", "nozh"), ("cand1108", "cand1108z")):
        xs = [(norm(R[k][a]), norm(R[k][b])) for k in ks]
        if all(x["solved"] and y["solved"] for x, y in xs):
            q = [(y["t"] + 10) / (x["t"] + 10) for x, y in xs]
            if all(v >= 1.5 for v in q) or all(v <= 1 / 1.5 for v in q):
                print(f"  {a:8s} → {b:9s} {p:28s} " + "  ".join(f"{x['t']:.1f} → {y['t']:.1f}" for x, y in xs))
print("\n名指しの問題:")
for p in ("ns1208400", "tbfp-network", "comp07-2idx", "misc07", "noswot", "eil33-2", "neos-860300"):
    ks = [k for k in keys if k[0] == p]
    for k in ks:
        print(f"  {p:16s} 種{k[1]} " + "  ".join(f"{n} {desc(norm(R[k][n]))}" for n in names))
print("\n食い違い:")
bad = 0
for k in keys:
    xs = {n: norm(R[k][n]) for n in names}
    if k[0] in H: xs["highs"] = norm(H[k[0]])
    opts = [(n, x["obj"]) for n, x in xs.items() if x["solved"] and x["st"].startswith("optimal") and x["obj"] is not None]
    for i in range(len(opts)):
        for j in range(i + 1, len(opts)):
            if abs(opts[i][1] - opts[j][1]) > 1e-6 * max(1.0, abs(opts[i][1])):
                print(f"  {k} 目的値の不一致 {opts[i]} 対 {opts[j]}"); bad += 1
    incs = [(n, x["obj"]) for n, x in xs.items() if x["obj"] is not None]
    for n, x in xs.items():
        if n == "highs": continue
        for nn, z in incs:
            if x["bd"] is not None and x["bd"] > z + 1e-6 * max(1.0, abs(z)):
                print(f"  {k} {n} の下界 {x['bd']:.10g} が {nn} の暫定解 {z:.10g} を越える"); bad += 1
print(f"  計 {bad} 件")

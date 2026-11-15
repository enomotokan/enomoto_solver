"""Mittelmann 186 問の 600 秒 res9 (2026-11-11、今の既定 = cand1108zd + DRCR + SYMDYN=f、固定のコミット f50eba16) の集計。
比べる相手: res6 (2026-11-08) の既定 ("ours"、cand1108zd の前) と cand1108z ("ours:cand1108z"、cand1108zd とほぼ同じ)、HiGHS (res_s*.jsonl の 1 回)。
別の日・別の機械の回どうしなので、1〜2 割の差は時期・機械の違いを含みうる。数え方は cmp600g.py と同じ。使い方: python cmp600i.py"""
import json, math, glob
from pathlib import Path
D = Path(__file__).resolve().parent
TL = 600.0
R = {}
for s in (1, 2):
    for tag, sol, name in (("res6", "ours", "res6 既定"), ("res7", "ours:cand1108z", "res7 cand1108z")):
        for f in sorted(glob.glob(str(D / f"{tag}s{s}_s*.jsonl"))):
            for l in open(f):
                r = json.loads(l)
                if r["solver"] == sol:
                    R.setdefault((r["problem"], s), {})[name] = r
for f in sorted(glob.glob(str(D / "res9_s*.jsonl"))):
    for l in open(f):
        r = json.loads(l)
        R.setdefault((r["problem"], r["seed"]), {})["res9 今の既定"] = r
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
    return dict(solved=solved, t=min(t, TL) if solved else TL, st=st, obj=obj if fin(obj) else None, bd=bd if fin(bd) else None)
def sgm(ts): return math.exp(sum(math.log(t + 10) for t in ts) / len(ts)) - 10
names = ["res6 既定", "res7 cand1108z", "res9 今の既定"]
keys = sorted(k for k, v in R.items() if all(n in v for n in names))
hs = {p for p, r in H.items() if norm(r)["solved"]}
print(f"3 つそろった (問題, 種): {len(keys)}。HiGHS が解いた問題: {len(hs)} / {len(H)}、HiGHS の SGM {sgm([norm(r)['t'] for r in H.values()]):.2f}")
for s in (1, 2, None):
    ks = [k for k in keys if s is None or k[1] == s]
    lab = f"種 {s}" if s else "2 種"
    base = sgm([norm(R[k][names[0]])["t"] for k in ks])
    parts = []
    for n in names:
        xs = [norm(R[k][n]) for k in ks]
        ho = sum(1 for k in ks if k[0] in hs and not norm(R[k][n])["solved"])
        oo = sum(1 for k in ks if k[0] not in hs and norm(R[k][n])["solved"])
        parts.append(f"{n} 解けた {sum(x['solved'] for x in xs)} SGM {sgm([x['t'] for x in xs]):.2f} (比 {sgm([x['t'] for x in xs]) / base:.3f}) HiGHSだけ {ho} ENOMOTOだけ {oo}")
    print(f"{lab} ({len(ks)}):\n  " + "\n  ".join(parts))
print("\nres6 既定と比べて 2 種とも解けたかが変わった問題 (res9):")
for p in sorted({k[0] for k in keys}):
    ks = [k for k in keys if k[0] == p]
    a = [norm(R[k][names[0]])["solved"] for k in ks]; b = [norm(R[k][names[2]])["solved"] for k in ks]
    if len(ks) == 2 and all(y and not x for x, y in zip(a, b)): print(f"  解けるようになった: {p} (" + "・".join(f"{norm(R[k][names[2]])['t']:.0f}" for k in ks) + " 秒)")
    if len(ks) == 2 and all(x and not y for x, y in zip(a, b)): print(f"  解けなくなった:     {p} (res6 " + "・".join(f"{norm(R[k][names[0]])['t']:.0f}" for k in ks) + " 秒)")
print("\nHiGHS だけが解く問題 (res9 で 2 種とも解けない):")
print("  " + ", ".join(sorted(p for p in hs if all(not norm(R[k][names[2]])["solved"] for k in keys if k[0] == p) and any(k[0] == p for k in keys))))
print("\n食い違い (res9 と HiGHS・res7):")
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
    x = xs[names[2]]
    for nn, z in incs:
        if x["bd"] is not None and x["bd"] > z + 1e-6 * max(1.0, abs(z)):
            print(f"  {k} res9 の下界 {x['bd']:.10g} が {nn} の暫定解 {z:.10g} を越える"); bad += 1
print(f"  計 {bad} 件")
rss = [r["max_rss_mb"] for k in keys for n in names[2:] if (r := R[k][n]).get("max_rss_mb")]
print(f"\nres9 の最大 RSS: 最大 {max(rss)} MB、5 GB 超 {sum(1 for v in rss if v > 5000)} 回")

"""Mittelmann 186 問の 600 秒の比べ res6 の集計 (2026-11-07)。res6s1_s*.jsonl (種 1)・res6s2_s*.jsonl (種 2) の今日の既定 ("ours") と
候補の形 cand1108 ("ours:cand1108") を、種ごとと 2 種合わせて比べる。HiGHS (res_s*.jsonl、1 回) と比べた「HiGHS だけが解く問題」の数も出す。
数え方は cmp600d.py と同じ (解けた = 最適か実行不能を示し 605 秒以内、SGM はずらし 10 秒で解けなければ 600)。
使い方: python cmp600e.py"""
import json, math, glob
from pathlib import Path
D = Path(__file__).resolve().parent
TL = 600.0
A, B = "ours", "ours:cand1108"
R = {}  # R[種][問題][設定]
for s in (1, 2):
    for f in sorted(glob.glob(str(D / f"res6s{s}_s*.jsonl"))):
        for l in open(f):
            r = json.loads(l)
            R.setdefault(s, {}).setdefault(r["problem"], {})[r["solver"]] = r
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
    return dict(solved=solved, crash=st == "crash", t=min(t, TL) if solved else TL, st=st, obj=obj if fin(obj) else None,
                bd=bd if fin(bd) else None, gap=gap)

def sgm(ts): return math.exp(sum(math.log(t + 10) for t in ts) / len(ts)) - 10
def desc(x): return f"{x['t']:.1f}" if x["solved"] else ("TL" + (f" gap {x['gap']:.1e}" if not math.isinf(x["gap"]) else " 解なし"))

probs = sorted(set(R[1]) & set(R[2]))
print(f"問題数 {len(probs)}")
for s in (1, 2):
    a = [norm(R[s][p][A]) for p in probs]; b = [norm(R[s][p][B]) for p in probs]
    hs = [norm(H[p]) for p in probs if p in H]
    ho_a = sum(1 for p in probs if p in H and norm(H[p])["solved"] and not norm(R[s][p][A])["solved"])
    ho_b = sum(1 for p in probs if p in H and norm(H[p])["solved"] and not norm(R[s][p][B])["solved"])
    print(f"種 {s}: 既定 解けた {sum(x['solved'] for x in a)} SGM {sgm([x['t'] for x in a]):.2f}  | cand1108 解けた {sum(x['solved'] for x in b)} "
          f"SGM {sgm([x['t'] for x in b]):.2f} (比 {sgm([x['t'] for x in b]) / sgm([x['t'] for x in a]):.3f})  | HiGHS だけが解く: 既定 {ho_a}・cand1108 {ho_b}"
          f"  | crash {sum(x['crash'] for x in a)}・{sum(x['crash'] for x in b)}")
ta = [norm(R[s][p][A])["t"] for s in (1, 2) for p in probs]; tb = [norm(R[s][p][B])["t"] for s in (1, 2) for p in probs]
print(f"2 種合わせて: 既定 SGM {sgm(ta):.2f}・cand1108 {sgm(tb):.2f} (比 {sgm(tb) / sgm(ta):.3f})")
hs_solved = sum(norm(H[p])["solved"] for p in probs if p in H)
print(f"HiGHS (1 回) 解けた {hs_solved}")

print("\n2 種とも cand1108 だけ解けた / 2 種とも既定だけ解けた:")
for p in probs:
    xs = [(norm(R[s][p][A]), norm(R[s][p][B])) for s in (1, 2)]
    if all(b["solved"] and not a["solved"] for a, b in xs): print(f"  cand1108 だけ: {p:28s} " + "  ".join(f"種{s} {desc(b)}" for s, (a, b) in zip((1, 2), xs)))
    if all(a["solved"] and not b["solved"] for a, b in xs): print(f"  既定だけ:     {p:28s} " + "  ".join(f"種{s} {desc(a)}" for s, (a, b) in zip((1, 2), xs)))
print("\n片方の種だけで解けたかが変わった:")
for p in probs:
    xs = [(norm(R[s][p][A]), norm(R[s][p][B])) for s in (1, 2)]
    ch = [a["solved"] != b["solved"] for a, b in xs]
    if sum(ch) == 1:
        print(f"  {p:28s} " + "  ".join(f"種{s} 既定 {desc(a)} / cand {desc(b)}" for s, (a, b) in zip((1, 2), xs)))
print("\n2 種とも同じ向きに 1.5 倍以上 (両方解けた回):")
for p in probs:
    xs = [(norm(R[s][p][A]), norm(R[s][p][B])) for s in (1, 2)]
    if all(a["solved"] and b["solved"] for a, b in xs):
        q = [(b["t"] + 10) / (a["t"] + 10) for a, b in xs]
        if all(v >= 1.5 for v in q) or all(v <= 1 / 1.5 for v in q):
            print(f"  {p:28s} " + "  ".join(f"種{s} {a['t']:.1f} → {b['t']:.1f}" for s, (a, b) in zip((1, 2), xs)))
print("\n2 種とも解けず、ギャップが 2 種とも同じ向きに 2 倍以上:")
for p in probs:
    xs = [(norm(R[s][p][A]), norm(R[s][p][B])) for s in (1, 2)]
    if all(not a["solved"] and not b["solved"] for a, b in xs):
        def better(a, b):  # b が良いと +1、悪いと -1
            if math.isinf(a["gap"]) and not math.isinf(b["gap"]): return 1
            if math.isinf(b["gap"]) and not math.isinf(a["gap"]): return -1
            if math.isinf(a["gap"]): return 0
            if b["gap"] < 0.5 * a["gap"] and a["gap"] > 1e-4: return 1
            if a["gap"] < 0.5 * b["gap"] and b["gap"] > 1e-4: return -1
            return 0
        v = [better(a, b) for a, b in xs]
        if v[0] == v[1] != 0:
            print(f"  {'良' if v[0] > 0 else '悪'} {p:26s} " + "  ".join(f"種{s} {desc(a)} → {desc(b)}" for s, (a, b) in zip((1, 2), xs)))
print("\nHiGHS だけが解く問題 (どちらかの種で):")
for name, key in (("既定", A), ("cand1108", B)):
    ps = [p for p in probs if p in H and norm(H[p])["solved"] and not any(norm(R[s][p][key])["solved"] for s in (1, 2))]
    print(f"  {name} (2 種とも解けない) {len(ps)}: {', '.join(ps)}")
print("\n食い違い:")
bad = 0
for p in probs:
    xs = {f"{k}@{s}": norm(R[s][p][k]) for s in (1, 2) for k in (A, B)}
    if p in H: xs["highs"] = norm(H[p])
    opts = [(k, x["obj"]) for k, x in xs.items() if x["solved"] and x["st"].startswith("optimal") and x["obj"] is not None]
    for i in range(len(opts)):
        for j in range(i + 1, len(opts)):
            if abs(opts[i][1] - opts[j][1]) > 1e-6 * max(1.0, abs(opts[i][1])):
                print(f"  {p:28s} 目的値の不一致 {opts[i][0]} {opts[i][1]:.10g} 対 {opts[j][0]} {opts[j][1]:.10g}"); bad += 1
    incs = [(k, x["obj"]) for k, x in xs.items() if x["obj"] is not None]
    for k, x in xs.items():
        if k == "highs": continue
        for kk, z in incs:
            if x["bd"] is not None and x["bd"] > z + 1e-6 * max(1.0, abs(z)):
                print(f"  {p:28s} {k} の下界 {x['bd']:.10g} が {kk} の暫定解 {z:.10g} を越える"); bad += 1
print(f"  計 {bad} 件")

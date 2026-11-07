"""Mittelmann 186 問の 600 秒の比べ res4 の集計 (2026-11-07)。res4_s*.jsonl の今日の既定 ("ours") と今日の候補を全部入れた形 ("ours:all1107") を比べ、
前日の既定 (res3_s*.jsonl) と HiGHS (res_s*.jsonl) の数も並べる。数え方は cmp600c.py と同じ: 解けた = 最適か実行不能を示し 605 秒以内、
SGM はずらし 10 秒 (解けなければ 600)、両方解けたら (t+10) の比 0.9 未満/超で勝ち負け、両方解けなければギャップが半分未満で勝ち。
食い違い: 両方最適で目的値が違う (相対 1e-6 超)、ある回の下界が別の回 (どの設定でも) の暫定解を越える (最小化、相対 1e-6 超)。
使い方: python cmp600d.py"""
import json, math, glob
from pathlib import Path
D = Path(__file__).resolve().parent
TL = 600.0
A, B = "ours", "ours:all1107"
R = {}
for f in sorted(glob.glob(str(D / "res4_s*.jsonl"))):
    for l in open(f):
        r = json.loads(l)
        R.setdefault(r["problem"], {})[r["solver"]] = r
for f in sorted(glob.glob(str(D / "res3_s*.jsonl"))):
    for l in open(f):
        r = json.loads(l)
        R.setdefault(r["problem"], {})["res3"] = r
for f in sorted(glob.glob(str(D / "res_s*.jsonl"))):
    for l in open(f):
        r = json.loads(l)
        if r["solver"] == "highs":
            R.setdefault(r["problem"], {})["highs"] = r

def fin(v): return v is not None and isinstance(v, (int, float)) and math.isfinite(v)

def norm(r):
    x = r["result"]; st = str(x.get("status", "")).lower()
    t = x.get("time")
    solved = (st.startswith("optimal") or st.startswith("infeasible")) and t is not None and t <= TL + 5
    obj, bd = x.get("objective"), x.get("bound")
    gap = abs(obj - bd) / max(abs(obj), 1.0) if fin(obj) and fin(bd) else math.inf
    if solved: gap = 0.0
    return dict(solved=solved, crash=st == "crash", t=min(t, TL) if solved else TL, st=st, obj=obj if fin(obj) else None,
                bd=bd if fin(bd) else None, gap=gap, rss=r.get("max_rss_mb", 0), root=x.get("root_bound"))

def sgm(ts): return math.exp(sum(math.log(t + 10) for t in ts) / len(ts)) - 10

def desc(x):
    if x["solved"]: return f"{x['t']:.1f}"
    return x["st"] + (f" gap {x['gap']:.2e}" if not math.isinf(x["gap"]) else " 解なし")

rows = [(p, norm(R[p][A]), norm(R[p][B])) for p in sorted(R) if A in R[p] and B in R[p]]
n = len(rows)
print(f"問題数 {n} (res4 の両方の設定がそろった問題)")
for name, key in (("今日の既定", A), ("all1107", B), ("前日の既定 res3", "res3"), ("HiGHS", "highs")):
    xs = [norm(R[p][key]) for p, _, _ in rows if key in R[p]]
    print(f"  {name:16s} 問題 {len(xs):3d}  解けた {sum(x['solved'] for x in xs):3d}  SGM {sgm([x['t'] for x in xs]):7.2f}  crash {sum(x['crash'] for x in xs)}")
both = [(p, a, b) for p, a, b in rows if a["solved"] or b["solved"]]
print(f"片方以上が解いた {len(both)} 問の SGM: 既定 {sgm([a['t'] for _, a, _ in both]):.2f}  all1107 {sgm([b['t'] for _, _, b in both]):.2f}"
      f"  比 {sgm([b['t'] for _, _, b in both]) / sgm([a['t'] for _, a, _ in both]):.3f}")
W = {"all1107": [], "既定": [], "互角": []}
for p, a, b in rows:
    if a["solved"] != b["solved"]:
        w = "all1107" if b["solved"] else "既定"
    elif a["solved"]:
        q = (b["t"] + 10) / (a["t"] + 10)
        w = "all1107" if q < 0.9 else "既定" if q > 1 / 0.9 else "互角"
    else:
        x, y = b["gap"], a["gap"]
        if math.isinf(x) and math.isinf(y): w = "互角"
        elif x < 0.5 * y: w = "all1107"
        elif y < 0.5 * x: w = "既定"
        else: w = "互角"
    W[w].append(p)
print("勝ち負け (all1107 から見て): " + "  ".join(f"{k} {len(v)}" for k, v in W.items()))
print("\n片方だけ解けた:")
for p, a, b in rows:
    if a["solved"] != b["solved"]:
        print(f"  {p:28s} 既定 {desc(a):>28s}  all1107 {desc(b):>28s}")
print("\n両方解けて 1.5 倍以上 (かつ 10 秒以上) 違う:")
for p, a, b in rows:
    if a["solved"] and b["solved"] and max(a["t"], b["t"]) >= 10 and max(a["t"] + 10, b["t"] + 10) / min(a["t"] + 10, b["t"] + 10) >= 1.5:
        print(f"  {p:28s} 既定 {a['t']:7.1f}  all1107 {b['t']:7.1f}")
print("\n両方解けずギャップが 2 倍以上違う (片方が解なしを含む):")
for p, a, b in rows:
    if not a["solved"] and not b["solved"]:
        x, y = a["gap"], b["gap"]
        if (math.isinf(x) != math.isinf(y)) or (not math.isinf(x) and max(x, y) > 2 * min(x, y) and max(x, y) > 1e-4):
            print(f"  {p:28s} 既定 {desc(a):>28s}  all1107 {desc(b):>28s}")
print("\ncrash:")
for p, a, b in rows:
    for s, x in (("既定", a), ("all1107", b)):
        if x["crash"]: print(f"  {p:28s} {s} 最大 RSS {x['rss']} MB")
print("\n食い違い:")
bad = 0
for p in sorted(R):
    xs = {k: norm(v) for k, v in R[p].items() if k in (A, B, "res3", "highs")}
    opts = [(k, x["obj"]) for k, x in xs.items() if x["solved"] and x["st"].startswith("optimal") and x["obj"] is not None]
    for i in range(len(opts)):
        for j in range(i + 1, len(opts)):
            if abs(opts[i][1] - opts[j][1]) > 1e-6 * max(1.0, abs(opts[i][1])):
                print(f"  {p:28s} 目的値の不一致 {opts[i][0]} {opts[i][1]:.10g} 対 {opts[j][0]} {opts[j][1]:.10g}"); bad += 1
    incs = [(k, x["obj"]) for k, x in xs.items() if x["obj"] is not None]
    for k, x in xs.items():
        if k not in (A, B): continue
        for kk, z in incs:
            if x["bd"] is not None and x["bd"] > z + 1e-6 * max(1.0, abs(z)):
                print(f"  {p:28s} {k} の下界 {x['bd']:.10g} が {kk} の暫定解 {z:.10g} を越える"); bad += 1
print(f"  計 {bad} 件")

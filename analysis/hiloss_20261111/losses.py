"""HILOSS: HiGHS に負けている問題の一覧 (Mittelmann 186 問、600 秒・4 スレッド)。
元: res9 (今の既定、種 1・2、analysis/mitt600_20261103/res9_s*.jsonl)、HiGHS (同 res_s*.jsonl の 1 回)、
cand1111 の Mittelmann の回 (analysis/cand1111_20261111/res_M_s*.jsonl、def と cand1111 の対、揃った分だけ)。
負けの大きさ = 種ごとの (ENOMOTO の時間 / HiGHS の時間) の幾何平均 (解けない回は 600 秒と数える、時間は +1 秒ずらす)。
使い方: python losses.py > losses.txt"""
import json, math, glob
from pathlib import Path
D = Path(__file__).resolve().parent
A = D.parent
TL = 600.0
def fin(v): return isinstance(v, (int, float)) and math.isfinite(v)
def norm(r):
    x = r["result"]; st = str(x.get("status", "")).lower(); t = x.get("time")
    solved = (st.startswith("optimal") or st.startswith("infeasible")) and t is not None and t <= TL + 5
    return dict(solved=solved, t=min(t, TL) if solved else TL, obj=x.get("objective") if fin(x.get("objective")) else None,
                bd=x.get("bound") if fin(x.get("bound")) else None, nodes=x.get("nodes"), rb=x.get("root_bound"), fst=x.get("first_solution_time"))
H, R9, C = {}, {}, {}
for f in glob.glob(str(A / "mitt600_20261103/res_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["solver"] == "highs": H[r["problem"]] = norm(r)
for f in glob.glob(str(A / "mitt600_20261103/res9_s*.jsonl")):
    for l in open(f):
        r = json.loads(l); R9[(r["problem"], r["seed"])] = norm(r)
for f in glob.glob(str(A / "cand1111_20261111/res_M_s*.jsonl")):
    for l in open(f):
        r = json.loads(l); C[(r["problem"], r["cfg"], r["seed"])] = norm(r)
def ratio(ts, th): return math.exp(sum(math.log((t + 1) / (th + 1)) for t in ts) / len(ts))
def cell(x):
    if x is None: return "  -  "
    return f"{x['t']:5.0f}" if x["solved"] else "   TL"
rows = []
for p, h in H.items():
    e = [R9.get((p, s)) for s in (1, 2)]
    if None in e or not h["solved"]: continue
    ns = sum(x["solved"] for x in e)
    rt = ratio([x["t"] for x in e], h["t"])
    if ns < 2 or rt >= 2.0:
        rows.append((0 if ns == 0 else 1 if ns == 1 else 2, -rt, p, h, e))
rows.sort()
print(f"cand1111 の Mittelmann の回: {len(C)} 行 (全 744)")
print("組: A = 2 種とも HiGHS だけ / B = 片方の種で HiGHS だけ / C = 2 種とも解くが HiGHS が 2 倍以上速い")
print(f"{'組':2} {'問題':28} {'HiGHS':>6} {'res9 s1':>7} {'s2':>5} {'比':>6} | {'c.def s1':>8} {'s2':>5} {'c1111 s1':>8} {'s2':>5} | res9 の終わりのギャップ% (s1,s2)  ノード (HiGHS / res9 s1)")
for g, nr, p, h, e in rows:
    cs = [C.get((p, c, s)) for c in ("def", "cand1111") for s in (1, 2)]
    gaps = []
    for x in e:
        if x["solved"]: gaps.append("解")
        elif x["obj"] is None: gaps.append("解なし")
        elif x["bd"] is None: gaps.append("?")
        else: gaps.append(f"{100 * abs(x['obj'] - x['bd']) / max(1e-9, abs(x['obj'])):.2f}")
    print(f"{'ABC'[g]:2} {p:28} {h['t']:6.0f} {cell(e[0]):>7} {cell(e[1]):>5} {-nr:6.1f} | {cell(cs[0]):>8} {cell(cs[1]):>5} {cell(cs[2]):>8} {cell(cs[3]):>5} | {gaps[0]:>7} {gaps[1]:>7}  {h['nodes']} / {e[0]['nodes']}")
# cand1111 で 2 種とも解けるようになる / ならない
print("\ncand1111 (揃った分) で HiGHS だけが解く問題 (2 種とも揃っていて 2 種とも解けない):")
print("  " + ", ".join(sorted(p for p, h in H.items() if h["solved"] and all((p, "cand1111", s) in C and not C[(p, "cand1111", s)]["solved"] for s in (1, 2)))))
print("ENOMOTO (res9) だけが解く問題 (どちらかの種):")
print("  " + ", ".join(sorted(p for p, h in H.items() if not h["solved"] and any(R9.get((p, s), {}).get("solved") for s in (1, 2)))))

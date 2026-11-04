"""Mittelmann 186 問の 600 秒の比べの集計 (2026-11-04)。res_s*.jsonl を読み、解けた数・SGM (ずらし 10 秒、時間切れ・crash は 600)・勝ち負け・
crash・最大 RSS・食い違いを出す。解けた = 最適か実行不能を示し、時間が 605 秒以内 (期限の後の後始末の分をわずかに許す)。
勝ち負け: 解けた方の勝ち。両方解けたら (t+10) の比が 0.9 未満/1/0.9 超で勝ち負け、間は互角。両方解けなければギャップ (相対、暫定解なしは無限大) が
半分未満で勝ち、どちらも同じくらい (比 0.5〜2、または両方無限大) なら互角。使い方: python cmp600.py"""
import json, math, glob
from pathlib import Path
D = Path(__file__).resolve().parent
R = {}
for f in sorted(glob.glob(str(D / "res_s*.jsonl"))):
    for l in open(f):
        r = json.loads(l)
        R.setdefault(r["problem"], {})[r["solver"]] = r
TL = 600.0
def norm(r):
    x = r["result"]; st = str(x.get("status", "")).lower()
    t = x.get("time")
    solved = (st.startswith("optimal") or st.startswith("infeasible")) and t is not None and t <= TL + 5
    crash = st == "crash"
    obj, bd = x.get("objective"), x.get("bound")
    def fin(v): return v is not None and isinstance(v, (int, float)) and math.isfinite(v)
    gap = abs(obj - bd) / max(abs(obj), 1.0) if fin(obj) and fin(bd) else math.inf
    if solved: gap = 0.0
    return dict(solved=solved, crash=crash, t=min(t, TL) if solved else TL, st=st, obj=obj if fin(obj) else None, gap=gap,
                rss=r.get("max_rss_mb", 0), rc=x.get("returncode"), rawt=t)
rows = []
for p in sorted(R):
    if "ours" not in R[p] or "highs" not in R[p]:
        print("欠け", p, list(R[p])); continue
    rows.append((p, norm(R[p]["ours"]), norm(R[p]["highs"])))
n = len(rows)
def sgm(ts): return math.exp(sum(math.log(t + 10) for t in ts) / len(ts)) - 10
so = [o["t"] for _, o, _ in rows]; sh = [h["t"] for _, _, h in rows]
print(f"問題数 {n}")
print(f"解けた数: ENOMOTO {sum(o['solved'] for _, o, _ in rows)}  HiGHS {sum(h['solved'] for _, _, h in rows)}")
print(f"SGM (ずらし 10 秒): ENOMOTO {sgm(so):.2f}  HiGHS {sgm(sh):.2f}  比 {sgm(so)/sgm(sh):.3f}")
both = [(p, o, h) for p, o, h in rows if o["solved"] or h["solved"]]
print(f"片方以上が解いた {len(both)} 問の SGM: ENOMOTO {sgm([o['t'] for _, o, _ in both]):.2f}  HiGHS {sgm([h['t'] for _, _, h in both]):.2f}")
W = {"ENOMOTO": [], "HiGHS": [], "互角": []}
for p, o, h in rows:
    if o["solved"] != h["solved"]:
        w = "ENOMOTO" if o["solved"] else "HiGHS"
    elif o["solved"]:
        q = (o["t"] + 10) / (h["t"] + 10)
        w = "ENOMOTO" if q < 0.9 else "HiGHS" if q > 1 / 0.9 else "互角"
    else:
        a, b = o["gap"], h["gap"]
        if math.isinf(a) and math.isinf(b): w = "互角"
        elif a < 0.5 * b: w = "ENOMOTO"
        elif b < 0.5 * a: w = "HiGHS"
        else: w = "互角"
    W[w].append(p)
print("勝ち負け: " + "  ".join(f"{k} {len(v)}" for k, v in W.items()))
ws = {k: [] for k in W}
for p, o, h in rows:
    if o["solved"] and h["solved"]:
        q = (o["t"] + 10) / (h["t"] + 10); ws["ENOMOTO" if q < 0.9 else "HiGHS" if q > 1 / 0.9 else "互角"].append(p)
print("両方解けた問題の勝ち負け: " + "  ".join(f"{k} {len(v)}" for k, v in ws.items()))
print("\n片方だけ解けた:")
for p, o, h in rows:
    if o["solved"] != h["solved"]:
        print(f"  {p:28s} ENOMOTO {('%.1f' % o['t']) if o['solved'] else o['st'] + (' gap %.2e' % o['gap'] if not math.isinf(o['gap']) else '')}"
              f"  HiGHS {('%.1f' % h['t']) if h['solved'] else h['st'] + (' gap %.2e' % h['gap'] if not math.isinf(h['gap']) else '')}")
print("\ncrash:")
for p, o, h in rows:
    for s, x in (("ENOMOTO", o), ("HiGHS", h)):
        if x["crash"]: print(f"  {p:28s} {s} returncode {x['rc']} 最大 RSS {x['rss']} MB")
print("\n600 秒を 5 秒超えて返った回:")
for p, o, h in rows:
    for s, x in (("ENOMOTO", o), ("HiGHS", h)):
        if x["rawt"] is not None and x["rawt"] > TL + 5: print(f"  {p:28s} {s} {x['st']} {x['rawt']:.1f}")
print("\n食い違い (両方最適で目的値が相対 1e-6 を超えて違う、片方だけ実行不能):")
bad = 0
for p, o, h in rows:
    if o["solved"] and h["solved"]:
        if o["st"].startswith("infeasible") != h["st"].startswith("infeasible"):
            print(f"  {p} 状態 {o['st']} 対 {h['st']}"); bad += 1
        elif o["obj"] is not None and h["obj"] is not None and abs(o["obj"] - h["obj"]) > 1e-6 * max(1, abs(h["obj"])):
            print(f"  {p} {o['obj']} 対 {h['obj']}"); bad += 1
print(f"  計 {bad}")
rs = sorted((o["rss"] / max(h["rss"], 1), p, o["rss"], h["rss"]) for p, o, h in rows if o["rss"] and h["rss"])
med = rs[len(rs) // 2][0]
print(f"\n最大 RSS の比 (ENOMOTO / HiGHS): 中央値 {med:.2f}、ENOMOTO が 5 GB 超: {sum(1 for _, o, _ in rows if o['rss'] > 5000)} 問、HiGHS が 5 GB 超: {sum(1 for _, _, h in rows if h['rss'] > 5000)} 問")
print("  比の大きい 8 問: " + ", ".join(f"{p} {a}/{b}MB" for q, p, a, b in rs[-8:]))
print("\n勝ち (ENOMOTO): " + ", ".join(W["ENOMOTO"]))
print("\n負け (HiGHS): " + ", ".join(W["HiGHS"]))

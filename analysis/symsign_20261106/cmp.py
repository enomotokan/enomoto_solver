"""bench_<名前>.jsonl の集計: A と B (既定 def と both) の SGM (ずらし 10 秒)・解けた数 (最適か実行不能を時間内に)・片方だけ解けた回・
最適値の食い違い・2 倍以上の差。使い方: python cmp.py 名前 [A] [B]"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
name = sys.argv[1]; A = sys.argv[2] if len(sys.argv) > 2 else "def"; B = sys.argv[3] if len(sys.argv) > 3 else "both"
R = defaultdict(dict); TL = {}
for l in open(D / f"bench_{name}.jsonl"):
    r = json.loads(l); R[(r["problem"], r["rep"])][r["cfg"]] = r["result"]; TL[r["problem"]] = r["tl"]
def ok(res, tl): return res.get("status") in ("optimal", "infeasible") and res.get("time", 1e9) <= tl
def t(res, tl): return res["time"] if ok(res, tl) else tl
def sgm(xs): return math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
def row(keys, label):
    keys = [k for k in keys if A in R[k] and B in R[k]]
    if not keys: return
    sa, sb = sgm([t(R[k][A], TL[k[0]]) for k in keys]), sgm([t(R[k][B], TL[k[0]]) for k in keys])
    na, nb = sum(ok(R[k][A], TL[k[0]]) for k in keys), sum(ok(R[k][B], TL[k[0]]) for k in keys)
    print(f"{label:12s} n={len(keys):3d}  {A}: {sa:6.2f}s ({na})  {B}: {sb:6.2f}s ({nb})  比 {sb / sa - 1:+.1%}")
reps = sorted({k[1] for k in R})
for rep in reps: row([k for k in R if k[1] == rep], f"rep {rep}")
row(list(R), "全回")
print("\n片方だけ解けた回・2 倍以上の差 (どちらも 5 秒以上の側があるもの)・最適値の食い違い・異常:")
for k in sorted(R):
    d = R[k]
    if A not in d or B not in d: continue
    a, b, tl = d[A], d[B], TL[k[0]]
    f = lambda r: f"{r['time']:.1f}" if ok(r, tl) else f"TL({r.get('status')})"
    note = []
    if ok(a, tl) != ok(b, tl): note.append("片方だけ")
    elif ok(a, tl) and max(a["time"], b["time"]) >= 5 and max(a["time"], b["time"]) >= 2 * min(a["time"], b["time"]): note.append("2倍以上")
    if a.get("status") == "optimal" and b.get("status") == "optimal" and abs(a["objective"] - b["objective"]) > 1e-6 * max(1, abs(a["objective"])): note.append(f"最適値 {a['objective']} vs {b['objective']}")
    if {a.get("status"), b.get("status")} & {"crash", "error"}: note.append("異常")
    if note: print(f"  {k[0]:28s} r{k[1]}  {A} {f(a):>12s}  {B} {f(b):>12s}  {' '.join(note)}")

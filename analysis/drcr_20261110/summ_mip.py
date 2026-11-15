"""MIP の計測の集計 (analysis/drcr_20261110.md)。python summ_mip.py <jsonl> [設定,...]
解けた = optimal かつ上限以内、時間は解けなければ上限。SGM のずらし 10 秒。誤答の検査: optimal の目的値を既知の最適値
(scripts/miplib2017-v31.solu の =opt=) と、同じ問題の既定の optimal と比べる (相対 1e-6)。"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
WT = Path(__file__).resolve().parents[2]
solu = {}
for l in open(WT / "scripts" / "miplib2017-v31.solu"):
    p = l.split()
    if len(p) >= 3 and p[0] in ("=opt=", "=best="):
        solu[p[1]] = (p[0], float(p[2]))
R = defaultdict(dict)
for l in open(sys.argv[1]):
    r = json.loads(l); R[(r["problem"], r["seed"])][r["cfg"]] = r
cfgs = sys.argv[2].split(",") if len(sys.argv) > 2 else ["def", "rfold", "rifold"]
def tt(r):
    res = r["result"]; tl = r["tl"]
    return res["time"] if res.get("status") == "optimal" and res.get("time", 1e9) <= tl else tl
def sgm(xs, s=10.0): return math.exp(sum(math.log(x + s) for x in xs) / len(xs)) - s
def close(a, b): return abs(a - b) <= 1e-6 * max(1.0, abs(a), abs(b))
wrong = []; safety = []; lifted_bad = []
for (p, s), v in sorted(R.items()):
    for c, r in v.items():
        res = r["result"]
        for l in r.get("fold", []):
            if "safety" in l: safety.append((p, s, c, l[:150]))
        if res.get("status") == "optimal" and res.get("objective") is not None:
            o = res["objective"]
            if p in solu and solu[p][0] == "=opt=" and not close(o, solu[p][1]): wrong.append((p, s, c, o, "solu", solu[p][1]))
            d = v.get("def", {}).get("result", {})
            if c != "def" and d.get("status") == "optimal" and not close(o, d["objective"]): wrong.append((p, s, c, o, "def", d["objective"]))
        # 下界が既知の最適値を超えていないか
        b = res.get("bound")
        if b is not None and p in solu and solu[p][0] == "=opt=" and isinstance(b, (int, float)) and math.isfinite(b):
            sense_ok = True
            o = solu[p][1]
            obj = res.get("objective")
            # 最小化を仮定せず、目的値と下界の向きから判断する (bound が opt を目的の向きに越えたら誤り)
            if obj is not None and ((obj >= b and b > o + 1e-6 * max(1, abs(o))) or (obj < b and b < o - 1e-6 * max(1, abs(o)))):
                wrong.append((p, s, c, b, "bound", o))
print(f"回 {len(R)}、誤答 (目的値・下界) {len(wrong)}、安全網 {len(safety)}")
for w in wrong: print("  誤答?", w)
for x in safety: print("  安全網", x)
keys = [k for k, v in R.items() if all(c in v for c in cfgs)]
probs = sorted({k[0] for k in keys})
print(f"全部の設定がそろった (問題, 種) {len(keys)} (問題 {len(probs)})")
for c in cfgs:
    ts = [tt(R[k][c]) for k in keys]
    print(f"  {c:7s} 解けた {sum(1 for k in keys if tt(R[k][c]) < R[k][c]['tl'])}  SGM {sgm(ts):.2f}  ({sgm(ts) + 10:.2f}/{sgm([tt(R[k]['def']) for k in keys]) + 10:.2f} = {(sgm(ts) + 10) / (sgm([tt(R[k]['def']) for k in keys]) + 10):.3f})")
print()
print("| 問題 | " + " | ".join(f"{c} 種1 / 種2" for c in cfgs) + " | 畳み |")
print("|---|" + "---|" * (len(cfgs) + 1))
for p in probs:
    cells = []
    for c in cfgs:
        cells.append(" / ".join(("%.1f" % tt(R[(p, s)][c]) if tt(R[(p, s)][c]) < R[(p, s)][c]["tl"] else "—") + ("" if R[(p, s)][c]["result"].get("status") == "optimal" else "") for s in (1, 2) if (p, s) in R and c in R[(p, s)]))
    fl = next((l for s in (1, 2) for c in cfgs if (p, s) in R and c in R[(p, s)] for l in R[(p, s)][c].get("fold", []) if "-> " in l and "n " in l), "")
    print(f"| {p} | " + " | ".join(cells) + f" | {fl[10:70]} |")

"""res8 (HiGHS に負ける 47 問、既定・umr・cand1108z・cand1108zd x 種 1・2、600 秒) を集計する。HiGHS は res_s*.jsonl の 1 回。
解けた = 最適か実行不能で 605 秒以内。SGM はずらし 10 秒、解けない回は 600 秒。使い方: python cmp600h.py > cmp600h.txt"""
import glob, json, math
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
probs = json.loads((D / "shards8.json").read_text())["problems"]
CF = [("既定", "ours"), ("umr", "ours:umr"), ("cand1108z", "ours:cand1108z"), ("cand1108zd", "ours:cand1108zd")]
def ent(x):
    st = (x.get("status") or "").lower(); t = x.get("time") or 600
    ok = st in ("optimal", "infeasible") and t <= 605
    return {"ok": ok, "t": t if ok else 600.0, "st": st, "bound": x.get("bound"), "obj": x.get("objective")}
H = {}
for f in glob.glob(str(D / "res_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["solver"] == "highs": H[r["problem"]] = ent(r["result"])
R = defaultdict(dict)
crash = 0
for f in glob.glob(str(D / "res8_s*.jsonl")):
    for l in open(f):
        if not l.strip(): continue
        r = json.loads(l); e = ent(r["result"]); R[(r["problem"], r["seed"])][r["solver"]] = e
        crash += e["st"] in ("crash", "fetch_error")
def sgm(ts): return math.exp(sum(math.log(t + 10) for t in ts) / len(ts)) - 10
def gap(e):
    b, o = e.get("bound"), e.get("obj")
    if b is None or o is None: return float("inf")
    return abs(o - b) / max(abs(o), abs(b), 1e-9)
keys = [(p, s) for p in probs for s in (1, 2) if all(c in R.get((p, s), {}) for _, c in CF)]
print(f"そろった (問題, 種): {len(keys)} / {2 * len(probs)}   crash・fetch_error: {crash}")
print("\n| 形 | 解けた | SGM (既定との比) | HiGHS より速い・同じ・遅い (解けた方の勝ち、両方解けたら (t+10) の比 0.9) | 2 種とも解けない |")
print("|---|---|---|---|---|")
base = sgm([R[k]["ours"]["t"] for k in keys]) if keys else 1
for name, c in CF:
    ts = [R[k][c]["t"] for k in keys]
    w = d = lo = 0
    for k in keys:
        a, h = R[k][c], H[k[0]]
        if a["ok"] and not h["ok"]: w += 1
        elif h["ok"] and not a["ok"]: lo += 1
        elif a["ok"] and h["ok"]:
            q = (a["t"] + 10) / (h["t"] + 10)
            w, d, lo = (w + 1, d, lo) if q < 0.9 else ((w, d, lo + 1) if q > 1 / 0.9 else (w, d + 1, lo))
        else: d += 1
    none2 = sum(1 for p in probs if all((p, s) in R and not R[(p, s)].get(c, {"ok": False})["ok"] for s in (1, 2)))
    print(f"| {name} | {sum(R[k][c]['ok'] for k in keys)} | {sgm(ts):.2f} ({sgm(ts) / base:.3f}) | {w}・{d}・{lo} | {none2} |" if keys else "")
print("\n問題ごと (秒、TL = 時間切れ (ギャップ))、HiGHS は res の 1 回:")
print("| 問題 | HiGHS | " + " | ".join(f"{n} 種1 | {n} 種2" for n, _ in CF) + " |")
print("|---|---|" + "---|---|" * len(CF))
def fmt(e):
    if e is None: return ""
    return f"{e['t']:.0f}" if e["ok"] else (f"TL({gap(e):.1e})" if e["st"] not in ("crash", "fetch_error") else e["st"])
for p in probs:
    cells = [fmt(R.get((p, s), {}).get(c)) for _, c in CF for s in (1, 2)]
    print(f"| {p} | {fmt(H[p])} | " + " | ".join(cells) + " |")

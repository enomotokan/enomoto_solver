"""TAB_HIGHS_RULES の問題ごとの比 (tab / def、shift 1 秒、時間切れは制限時間) を、revisit_20261112 (種 1・2) と
gap4_rootstuck_mem_20261109 (res58hr、rep 1・2) から集める。"""
import json, glob, math, collections
from pathlib import Path
A = Path(__file__).resolve().parents[2]
L = collections.defaultdict(list)
def ok(r): return r["status"] in ("optimal", "infeasible")
def t(r, tl): return min(r["time"], tl) if ok(r) else tl
R = {}
for f in glob.glob(str(A / "revisit_20261112/res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] in ("def", "tab"): R[("rv", r["problem"], r["cfg"], r["seed"])] = r["result"]
for l in open(A / "gap4_rootstuck_mem_20261109/res58hr.jsonl"):
    r = json.loads(l)
    if r["cfg"] in ("def", "hr"): R[("g4", r["problem"], "tab" if r["cfg"] == "hr" else "def", r["rep"])] = r["result"]
for (src, p, c, s), r in R.items():
    if c != "def": continue
    b = R.get((src, p, "tab", s))
    if not b: continue
    if ok(r) or ok(b): L[p].append(math.log((t(b, 60) + 1) / (t(r, 60) + 1)))
    else: L[p].append(None)
for p in sorted(L, key=lambda p: sum(x for x in L[p] if x is not None) / max(1, sum(1 for x in L[p] if x is not None))):
    v = [x for x in L[p] if x is not None]
    print(f"{p:22s} " + " ".join("  -  " if x is None else f"{math.exp(x):5.2f}" for x in L[p]) + (f"   平均 {math.exp(sum(v)/len(v)):.2f}" if v else ""))

"""58 問の結果 (results.jsonl の設定 def_b58・old_b58) を、既知の最適値と過去の計測 (bench8 の def) と比べる。
既知の最適値は scripts/miplib2017-v31.solu の =opt= (なければ optcheck.py の参照値)。「最適」の値が既知の値と相対 1e-4
(既定の相対ギャップ) を越えて違えば食い違い、時間切れの暫定解が既知の最適値より良ければ (最小化) 食い違いとする。
SGM はシフト 1 秒、時間切れ・60 秒超は 60 秒。使い方: python summ58.py [設定,...]"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
sys.path.insert(0, str(D))
from probs import PROBS
import optcheck
SOLU = {}
for l in open(WT / "scripts" / "miplib2017-v31.solu"):
    t = l.split()
    if len(t) >= 3 and t[0] == "=opt=":
        SOLU[t[1]] = float(t[2])
cfgs = sys.argv[1].split(",") if len(sys.argv) > 1 else ["def_b58", "old_b58"]
R = defaultdict(dict)
for l in open(D / "results.jsonl"):
    r = json.loads(l)
    if r["cfg"] in cfgs and r["res"]:
        R[r["cfg"]][r["prob"]] = r["res"]
# 過去の計測: bench8 の def (2 回の平均の時間)
B8 = defaultdict(list)
import glob
for f in glob.glob(str(WT / "analysis" / "halve_submip_20261030" / "bench8_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r.get("cfg") == "def" and r.get("result"):
            res = r["result"]
            B8[r["problem"]].append(res["time"] if res["status"] == "optimal" and res["time"] <= 60 else 60.0)

def t60(res):
    return res["time"] if res["status"] == "optimal" and res["time"] <= 60 else 60.0

def close(a, b, tol):
    return abs(a - b) <= tol * max(1.0, abs(a), abs(b))

print("| 問題 | 既知の最適値 | " + " | ".join(f"{c} 状態・値・時間" for c in cfgs) + " | bench8 def の時間 |")
print("|---|---|" + "---|" * len(cfgs) + "---|")
bad = []
for p in PROBS:
    ref = SOLU.get(p, optcheck.REF.get(p))
    cells = []
    for c in cfgs:
        res = R[c].get(p)
        if not res:
            cells.append("-")
            continue
        obj = res.get("objective")
        mark = ""
        if ref is not None and obj is not None:
            if res["status"] == "optimal" and not close(obj, ref, 1e-4):
                mark = " **食い違い**"
            elif res["status"] != "optimal" and obj < ref - 1e-6 * max(1.0, abs(ref)):
                mark = " **既知の最適値より良い**"
        if mark:
            bad.append((p, c, res["status"], obj, ref))
        cells.append(f"{res['status'][:4]} {obj if obj is None else f'{obj:.6g}'} {res['time']:.1f}{mark}")
    b8 = B8.get(p)
    print(f"| {p} | {ref if ref is None else f'{ref:.9g}'} | " + " | ".join(cells) + f" | {'・'.join(f'{x:.1f}' for x in b8) if b8 else '-'} |")

def sgm(v):
    return math.exp(sum(math.log(t + 1) for t in v) / len(v)) - 1
print()
common = [p for p in PROBS if all(p in R[c] for c in cfgs)]
for c in cfgs:
    v = [t60(R[c][p]) for p in common]
    print(f"{c}: {len(common)} 問の SGM {sgm(v):.2f}、解けた {sum(R[c][p]['status'] == 'optimal' and R[c][p]['time'] <= 60 for p in common)}")
cb = [p for p in common if B8.get(p)]
print(f"bench8 def ({len(cb)} 問、2 回の時間の平均): SGM {sgm([sum(B8[p]) / len(B8[p]) for p in cb]):.2f}")
for c in cfgs:
    print(f"  {c} の同じ {len(cb)} 問: SGM {sgm([t60(R[c][p]) for p in cb]):.2f}")
print(f"食い違い: {len(bad)}")
for b in bad:
    print("  ", *b)

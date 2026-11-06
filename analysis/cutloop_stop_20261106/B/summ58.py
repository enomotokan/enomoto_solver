"""res58.jsonl の集計 (G6 の summ58.py を写し、回 (rep) ごとに分けて数える。回の指定: python summ58.py res58.jsonl rep): 設定ごとの SGM (ずらし 10 秒、解けなければ 60 秒)・解けた数 (組ごと)、既定と 20% 以上かつ 2 秒以上違う問題、
食い違い (両方最適で目的値が違う) と異常終了。使い方: python summ58.py [res58.jsonl]"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parents[2] / "analysis" / "cbcimpl_20261029"))
from probs import SETS, PROBS  # noqa: E402
R = defaultdict(dict); cfgs = []
for l in open(sys.argv[1] if len(sys.argv) > 1 else D / "res58.jsonl"):
    r = json.loads(l)
    if len(sys.argv) > 2 and r["rep"] != int(sys.argv[2]): continue
    R[r["problem"]][r["cfg"]] = r["result"]
    if r["cfg"] not in cfgs: cfgs.append(r["cfg"])
def t(x): return min(x.get("time", 60), 60.0) if x.get("status") == "optimal" else 60.0
def sgm(xs): return math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
ps_all = [p for p in PROBS if all(c in R[p] for c in cfgs)]
print(f"両方そろった問題 {len(ps_all)}")
print("| 設定 | " + " | ".join(SETS) + " | 58 問 |"); print("|---|" + "---|" * (len(SETS) + 1))
for c in cfgs:
    cells = []
    for ps in list(SETS.values()) + [PROBS]:
        xs = [t(R[p][c]) for p in ps if p in ps_all]
        cells.append(f"{sgm(xs):.2f} 秒 ({sum(x < 60 for x in xs)}/{len(xs)})")
    print(f"| {c} | " + " | ".join(cells) + " |")
print("\n既定と 20% 以上かつ 2 秒以上違う問題 (秒、解けなければ 60 と下界・暫定解):")
for p in ps_all:
    b = t(R[p]["def"])
    for c in cfgs:
        if c == "def": continue
        v = t(R[p][c])
        if abs(v - b) > max(2.0, 0.2 * b):
            f = lambda x: f"{t(x):.1f}" + ("" if x.get("status") == "optimal" else f" [{x.get('bound')}, {x.get('objective')}]")
            print(f"- {p}: def {f(R[p]['def'])} / {c} {f(R[p][c])}")
print("\n食い違い・異常:")
for p in ps_all:
    opt = {c: R[p][c].get("objective") for c in cfgs if R[p][c].get("status") == "optimal"}
    if opt and max(opt.values()) - min(opt.values()) > 1e-6 * max(1, max(abs(v) for v in opt.values())):
        print(f"- {p}: {opt}")
    for c in cfgs:
        if R[p][c].get("status") not in ("optimal", "time_limit", "infeasible") or R[p][c].get("warn"):
            print(f"- {p} {c}: {R[p][c].get('status')} warn {R[p][c].get('warn')}")

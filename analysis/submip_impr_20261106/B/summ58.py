"""res58.jsonl の集計: 設定・rep ごとの SGM (ずらし 10 秒、解けなければ 60 秒)・解けた数、既定と 20% 以上かつ 2 秒以上違う問題
(両方の rep で同じ向きのものに印)、時間切れの問題の暫定解の比べ、食い違いと異常終了。使い方: python summ58.py"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parents[2] / "analysis" / "cbcimpl_20261029"))
from probs import SETS, PROBS  # noqa: E402
R = defaultdict(dict); keys = []
for l in open(D / "res58.jsonl"):
    r = json.loads(l); k = (r["cfg"], r["rep"]); R[r["problem"]][k] = r["result"]
    if k not in keys: keys.append(k)
keys.sort()
def t(x): return min(x.get("time", 60), 60.0) if x.get("status") == "optimal" else 60.0
def sgm(xs): return math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
ps = [p for p in PROBS if all(k in R[p] for k in keys)]
print(f"全ての回がそろった問題 {len(ps)}")
print("| 設定 | rep | " + " | ".join(SETS) + " | 58 問 |"); print("|---|---|" + "---|" * (len(SETS) + 1))
for k in keys:
    cells = []
    for s in list(SETS.values()) + [PROBS]:
        xs = [t(R[p][k]) for p in s if p in ps]
        cells.append(f"{sgm(xs):.2f} 秒 ({sum(x < 60 for x in xs)}/{len(xs)})")
    print(f"| {k[0]} | {k[1]} | " + " | ".join(cells) + " |")
cfgs = sorted({k[0] for k in keys}); reps = sorted({k[1] for k in keys})
f = lambda x: f"{t(x):.1f}" + ("" if x.get("status") == "optimal" else f" [{x.get('bound')}, {x.get('objective')}]")
for c in cfgs:
    if c == "def": continue
    print(f"\n{c} が既定と 20% 以上かつ 2 秒以上違う問題 (同じ rep どうし。秒、解けなければ 60 と [下界, 暫定解]):")
    for p in ps:
        diffs = []
        for r in reps:
            b, v = t(R[p][("def", r)]), t(R[p][(c, r)])
            diffs.append(0 if abs(v - b) <= max(2.0, 0.2 * b) else (1 if v > b else -1))
        if any(diffs):
            same = len(set(diffs)) == 1
            print(f"- {p}{' (両 rep で同じ向き)' if same else ''}: " + " / ".join(f"rep{r}: def {f(R[p][('def', r)])}, {c} {f(R[p][(c, r)])}" for r in reps))
    print(f"\n{c}: 両方時間切れの問題の暫定解 (最小化に直した値、def の 2 回 / {c} の 2 回):")
    better = worse = 0
    for p in ps:
        xs = [R[p][(cc, r)] for cc in ("def", c) for r in reps]
        if all(x.get("status") == "time_limit" for x in xs):
            ob = [x.get("objective") for x in xs]
            if None in ob: print(f"- {p}: {ob}"); continue
            n = len(reps); a, b = sorted(ob[:n]), sorted(ob[n:])
            # 最小化か最大化かは下界と比べて決める
            sense = 1 if (xs[0].get("bound") or -1e300) <= ob[0] else -1
            ma, mb = sum(a) / n * sense, sum(b) / n * sense
            tag = "" if abs(ma - mb) <= 1e-6 * max(1, abs(ma)) else ("  ← 試作が良い" if mb < ma else "  ← 試作が悪い")
            better += mb < ma - 1e-6 * max(1, abs(ma)); worse += mb > ma + 1e-6 * max(1, abs(ma))
            print(f"- {p}: def {a} / {c} {b}{tag}")
    print(f"  (試作が良い {better}、悪い {worse})")
print("\n食い違い・異常:")
for p in ps:
    opt = {k: R[p][k].get("objective") for k in keys if R[p][k].get("status") == "optimal"}
    if opt and max(opt.values()) - min(opt.values()) > 1e-6 * max(1, max(abs(v) for v in opt.values())):
        print(f"- {p}: {opt}")
    for k in keys:
        if R[p][k].get("status") not in ("optimal", "time_limit", "infeasible") or R[p][k].get("warn"):
            print(f"- {p} {k}: {R[p][k].get('status')} warn {R[p][k].get('warn')}")

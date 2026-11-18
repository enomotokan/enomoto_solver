"""parorder P の集計 (def と a・cg・p3 の対)。python summ.py P > summ_P.txt
SGM は shift 1 秒、時間切れは制限時間で数える。CI は問題単位のブートストラップ (slow2 の summ.py と同じ)。"""
import json, glob, math, random, collections, sys
from pathlib import Path
D = Path(__file__).resolve().parent
SET = sys.argv[1] if len(sys.argv) > 1 else "P"
rows = [json.loads(l) for f in sorted(glob.glob(str(D / f"res_{SET}_s*.jsonl"))) for l in open(f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
def st(r): return str(r["result"].get("status") or "").lower()
def solved(r): return st(r) in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
probs = sorted({p for p, c, s in R}); seeds = sorted({s for p, c, s in R}); cfgs = sorted({c for p, c, s in R})
crash = sum(1 for r in rows if "error" in r["result"] or st(r) in ("", "crash", "error"))
print(f"{SET}: 行数 {len(rows)}、問題 {len(probs)}、種 {seeds}、設定 {cfgs}、crash {crash}")
print("| 設定 | 解けた数 | SGM 設定/def [95% CI] | 全種で 1.25 倍超 | 食い違い |")
print("|---|---|---|---|---|")
for c in [x for x in cfgs if x != "def"]:
    full = [(p, s) for p in probs for s in seeds if (p, c, s) in R and (p, "def", s) in R]
    logr = collections.defaultdict(list); bad = []
    for p, s in full:
        a, b = R[(p, "def", s)], R[(p, c, s)]
        if st(a) == st(b) == "optimal" and abs(a["result"]["objective"] - b["result"]["objective"]) > 1e-6 * max(1, abs(a["result"]["objective"])): bad.append((p, s))
        if solved(a) or solved(b): logr[p].append(math.log((t(b) + 1) / (t(a) + 1)))
    ps = list(logr)
    def sgm(q):
        v = [x for p in q for x in logr[p]]; return math.exp(sum(v) / len(v)) if v else 1.0
    random.seed(0); bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000))
    slow = [p for p in ps if len(logr[p]) == len(seeds) and all(math.exp(x) > 1.25 for x in logr[p])]
    nd = sum(solved(R[(p, "def", s)]) for p, s in full); nc = sum(solved(R[(p, c, s)]) for p, s in full)
    print(f"| {c} | {nd} → {nc} | {sgm(ps)-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}] | {', '.join(slow) or 'なし'} | {len(bad)} |")
    for p in sorted(ps, key=lambda q: -abs(sum(logr[q]) / len(logr[q])))[:6]:
        print(f"    {c} {p}: {math.exp(sum(logr[p]) / len(logr[p])):.2f} {[round(math.exp(x), 2) for x in logr[p]]}")

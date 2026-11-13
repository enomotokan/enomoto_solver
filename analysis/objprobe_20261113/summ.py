"""OBJPROBE の集計: python summ.py [P|H] > summ_P.txt。base との対の比 (SGM、shift 1 秒、時間切れは制限時間)、問題単位のブートストラップの 95% 区間、
解けた数、食い違い (最適値の不一致・最適と実行不能の食い違い、4 設定の間)、種をまたいで揃って遅い/速い問題。H は問題ごとの表も出す。"""
import json, glob, math, random, collections, sys
from pathlib import Path
D = Path(__file__).resolve().parent
SET = sys.argv[1] if len(sys.argv) > 1 else "P"
rows = [json.loads(l) for f in sorted(glob.glob(str(D / f"res_{SET}_s*.jsonl"))) for l in open(f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
CF = [c for c in ("base", "c1", "c2", "c12", "c12h") if any(k[1] == c for k in R)]
def solved(r): return r["result"].get("status") in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
probs = sorted({p for p, c, s in R}); seeds = sorted({s for p, c, s in R})
crash = [k for k, r in R.items() if r["result"].get("status") in (None, "crash", "error", "fetch_error")]
bad = []
for p in probs:
    for s in seeds:
        rs = [(c, R[p, c, s]) for c in CF if (p, c, s) in R]
        opt = [(c, r["result"]["objective"]) for c, r in rs if r["result"].get("status") == "optimal"]
        inf = [c for c, r in rs if r["result"].get("status") == "infeasible"]
        if opt and inf: bad.append((p, s, "opt/inf", opt, inf))
        if opt and max(o for _, o in opt) - min(o for _, o in opt) > 1e-6 * max(1, abs(opt[0][1])): bad.append((p, s, opt))
        # 時間切れの暫定解・下界が他の設定の最適値と矛盾しないか
        for c, r in rs:
            for c2, o in opt:
                b = r["result"].get("bound"); z = r["result"].get("objective")
                if b is not None and b > o + 1e-6 * max(1, abs(o)): bad.append((p, s, c, "bound", b, c2, o))
                if z is not None and z < o - 1e-6 * max(1, abs(o)): bad.append((p, s, c, "obj", z, c2, o))
print(f"行数 {len(rows)}、crash {len(crash)} {crash[:5]}、食い違い {len(bad)} {bad[:10]}")
print("解けた数: " + ", ".join(f"{c} {sum(solved(r) for k, r in R.items() if k[1] == c)}" for c in CF))
for c in CF[1:]:
    logr = collections.defaultdict(list)
    for p in probs:
        for s in seeds:
            a, b = R.get((p, "base", s)), R.get((p, c, s))
            if a and b and (solved(a) or solved(b)): logr[p].append(math.log((t(b) + 1) / (t(a) + 1)))
    ps = list(logr)
    def sgm(pp):
        v = [x for q in pp for x in logr[q]]
        return math.exp(sum(v) / len(v)) if v else 1.0
    random.seed(0); bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000))
    print(f"{c} / base (どちらかが解けた {len(ps)} 問): {sgm(ps)-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}]")
    for p in ps:
        v = logr[p]
        if len(v) >= 2 and all(x > math.log(1.25) for x in v): print("   遅", p, [round(math.exp(x), 2) for x in v])
        if len(v) >= 2 and all(x < math.log(0.8) for x in v): print("   速", p, [round(math.exp(x), 2) for x in v])
if SET in ("H", "G"):
    print("\n| 問題 | " + " | ".join(f"{c} 状態 / 秒 / 暫定解 / 下界" for c in CF) + " |")
    print("|---|" + "---|" * len(CF))
    for p in probs:
        cells = []
        for c in CF:
            r = R.get((p, c, 1))
            if not r: cells.append("-"); continue
            x = r["result"]
            cells.append(f"{x.get('status')} / {x.get('time', 0):.0f} / {x.get('objective')} / {x.get('bound') if x.get('bound') is None else round(x['bound'], 4)}")
        print(f"| {p} | " + " | ".join(cells) + " |")

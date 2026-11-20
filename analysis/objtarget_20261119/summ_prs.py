"""PRS (README の 10 節の広い計測) の集計: python summ_prs.py > summ_PRS.txt。
{c, cb, pl, pl_cb} それぞれ def に対して: 解けた数、両方解けた組の秒の比の SGM (shift 1 秒、問題単位のブートストラップで 95% 区間)、
全種で 1.25 倍超遅い問題 (秒の大きい方が 1 秒以上)、指定の問題の種ごとの比と再スタートの回数、目的値の食い違い、時間切れの暫定解の差。"""
import json, glob, math, random, re, collections
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for f in sorted(glob.glob(str(D / "res_PRS_s*.jsonl"))) if re.fullmatch(r"res_PRS_s\d+\.jsonl", Path(f).name) for l in open(f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r["result"] for r in rows}
ALTS = ["c", "cb", "pl", "pl_cb"]
WATCH = ["10teams", "noswot", "nw04", "roll3000", "neos-911970"]
def solved(x): return x["status"] in ("optimal", "infeasible")
probs = sorted({p for p, c, s in R}); seeds = sorted({s for p, c, s in R})
crash = sum(1 for x in R.values() if x["status"] not in ("optimal", "infeasible", "time_limit"))
print(f"PRS: 行数 {len(rows)}、問題 {len(probs)}、種 {seeds}、crash {crash}")
def boot(logr):
    ps = list(logr)
    def g(q):
        v = [x for p in q for x in logr[p]]; return math.exp(sum(v) / len(v)) if v else 1.0
    random.seed(0); bs = sorted(g([random.choice(ps) for _ in ps]) for _ in range(4000))
    return g(ps), bs[100], bs[3899], len(ps)
pct = lambda v: f"{(v - 1) * 100:+.1f}%"
for a in ALTS:
    full = [(p, s) for p in probs for s in seeds if (p, "def", s) in R and (p, a, s) in R]
    nd = sum(solved(R[(p, "def", s)]) for p, s in full); na = sum(solved(R[(p, a, s)]) for p, s in full)
    logr = collections.defaultdict(list); one = []
    for p, s in full:
        x, y = R[(p, "def", s)], R[(p, a, s)]
        if solved(x) and solved(y): logr[p].append(math.log((y["time"] + 1) / (x["time"] + 1)))
        elif solved(x) != solved(y): one.append(f"{p} s{s} (def {x['status']} {x['time']:.1f} / {a} {y['status']} {y['time']:.1f})")
    g, lo, hi, n = boot(logr)
    print(f"\n== {a} / def: 組 {len(full)}、解けた数 def {nd} / {a} {na}、両方解けた {sum(len(v) for v in logr.values())} 組 ({n} 問)")
    print(f"  秒の SGM の比 {pct(g)} [{pct(lo)}, {pct(hi)}]")
    print("  片方だけ解けた:", one or "なし")
    slow = []
    for p in probs:
        rs = []
        for s in seeds:
            if (p, "def", s) not in R or (p, a, s) not in R: continue
            x, y = R[(p, "def", s)], R[(p, a, s)]
            tx = x["time"] if solved(x) else 60.0; ty = y["time"] if solved(y) else 60.0
            rs.append((ty / tx if tx > 0 else 1.0, max(tx, ty)))
        if rs and all(r > 1.25 and m >= 1 for r, m in rs): slow.append(f"{p} ({', '.join(f'x{r:.2f}' for r, _ in rs)})")
    print("  全種で 1.25 倍超遅い:", slow or "なし")
print("\n== 指定の問題: 種ごとの秒 (再スタートの回数)")
for p in WATCH:
    print(f"  {p}")
    for c in ["def"] + ALTS:
        print(f"    {c:6s} " + "  ".join(f"s{s} {R[(p, c, s)]['time']:5.1f}{'' if solved(R[(p, c, s)]) else 'TL'} r{R[(p, c, s)].get('restarts')}" for s in seeds if (p, c, s) in R))
print("\n== 目的値の食い違い (同じ問題で optimal の目的値が割れる、暫定解が他の設定の下界を割る)")
bad = []
for p in probs:
    objs = {round(x["objective"], 6) for (q, c, s), x in R.items() if q == p and x["status"] == "optimal"}
    if len(objs) > 1 and (max(objs) - min(objs)) > 1e-6 * max(1, abs(min(objs))): bad.append(f"{p} {sorted(objs)}")
    xs = [x for (q, c, s), x in R.items() if q == p]
    for x in xs:
        for y in xs:
            if x.get("objective") is not None and y.get("bound") is not None and x["objective"] < y["bound"] - 1e-6 * max(1, abs(y["bound"])): bad.append(f"{p} 暫定 {x['objective']} < 下界 {y['bound']}")
print("  ", bad or "なし")
print("\n== 時間切れの暫定解 (def と alt が両方時間切れの組、相対差 1e-4 超。正 = alt が悪い。最小化)")
for p in probs:
    for s in seeds:
        x = R.get((p, "def", s))
        if not x or x["status"] != "time_limit": continue
        out = []
        for a in ALTS:
            y = R.get((p, a, s))
            if not y or y["status"] != "time_limit": continue
            ox, oy = x.get("objective"), y.get("objective")
            if ox is None or oy is None:
                if ox != oy: out.append(f"{a} {oy}")
                continue
            d = (oy - ox) / max(1.0, abs(ox))
            if abs(d) > 1e-4: out.append(f"{a} {oy:.6g} ({d * 100:+.2f}%)")
        if out: print(f"  {p} s{s}: def {x.get('objective'):.6g} / " + ", ".join(out))

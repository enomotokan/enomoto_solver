"""SMALLFIX の集計: python summ.py > summ.txt
(P) 58 問 x 種 1〜3 の対の比 (c3・d1・c3d1 / base、SGM は shift 1 秒、時間切れは 60 秒で数える。CI は問題単位のブートストラップ)、解けた数、
食い違い (同じ (問題, 種) で最適値が違う・最適と実行不能)、種をまたいで揃って遅い/速い問題。木の中の再スタートの数 (base の回で、実際に行ったもの・
そのうち固定 0 列) と、d1 で止めた数。clique fallback の試しの秒 (base) と c3 で省いた・止めた数。
(H) 12 問 600 秒 種 1 の base 対 c3d1 の 1 行ずつ。"""
import json, glob, math, random, collections
from pathlib import Path
D = Path(__file__).resolve().parent
def load(s):
    return {(r["problem"], r["cfg"], r["seed"]): r for f in sorted(glob.glob(str(D / f"res_{s}_s*.jsonl"))) for l in open(f) for r in [json.loads(l)]}
def solved(r): return r["result"].get("status") in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
R = load("P")
probs = sorted({p for p, c, s in R}); seeds = sorted({s for p, c, s in R})
print(f"P: 行数 {len(R)}、問題 {len(probs)}、種 {seeds}")
crash = [k for k, r in R.items() if r["result"].get("status") in (None, "crash", "error", "fetch_error")]
print("crash", crash)
for c in ("c3", "d1", "c3d1"):
    logr = collections.defaultdict(list); ns = [0, 0]; bad = []
    for p in probs:
        for s in seeds:
            a, b = R.get((p, "base", s)), R.get((p, c, s))
            if not a or not b: continue
            ns[0] += solved(a); ns[1] += solved(b)
            sa, sb = a["result"].get("status"), b["result"].get("status")
            if sa == sb == "optimal":
                oa, ob = a["result"]["objective"], b["result"]["objective"]
                if abs(oa - ob) > 1e-6 * max(1, abs(oa)): bad.append((p, s, oa, ob))
            if {sa, sb} == {"optimal", "infeasible"}: bad.append((p, s, sa, sb))
            if solved(a) or solved(b): logr[p].append(math.log((t(b) + 1) / (t(a) + 1)))
    ps = list(logr)
    def sgm(pp):
        v = [x for p in pp for x in logr[p]]
        return math.exp(sum(v) / len(v)) if v else 1.0
    est = sgm(ps); random.seed(0)
    bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000))
    same = sum(1 for p in ps for x in logr[p] if abs(x) < 1e-9)
    print(f"\n== {c} / base: 解けた base {ns[0]} / {c} {ns[1]}、食い違い {len(bad)} {bad}")
    print(f"  対の比 (どちらかが解けた {len(ps)} 問): {est - 1:+.1%} [{bs[100] - 1:+.1%}, {bs[3899] - 1:+.1%}]")
    ch = [p for p in ps if any(abs(x) > math.log(1.1) for x in logr[p])]
    print(f"  比が 1.1 倍を超えて動いた回のある問題 {len(ch)}: " + "; ".join(f"{p} {[round(math.exp(x), 2) for x in logr[p]]}" for p in ch))
    for p in ps:
        v = logr[p]
        if len(v) >= 3 and all(x > math.log(1.25) for x in v): print("  揃って遅", p, [round(math.exp(x), 2) for x in v])
        if len(v) >= 3 and all(x < math.log(0.8) for x in v): print("  揃って速", p, [round(math.exp(x), 2) for x in v])
print("\n== 木の中の再スタート (cnt、回の数)")
for c in ("base", "c3", "d1", "c3d1"):
    rs = [r for k, r in R.items() if k[1] == c and "cnt" in r]
    tr = sum(r["cnt"]["tree_rst"] for r in rs); f0 = sum(r["cnt"]["tree_rst_fix0"] for r in rs); z = sum(r["cnt"]["trnofix_stop"] for r in rs)
    nr = sum(1 for r in rs if r["cnt"]["tree_rst"]); nf = sum(1 for r in rs if r["cnt"]["tree_rst_fix0"])
    print(f"  {c}: 回 {len(rs)}、再スタート {tr} (回 {nr})、うち固定 0 列 {f0} (回 {nf})、TRNOFIX で止めた {z}")
print("  base で固定 0 列の再スタートのあった (問題, 種):", sorted((k[0], k[2], r["cnt"]["tree_rst_fix0"]) for k, r in R.items() if k[1] == "base" and r.get("cnt", {}).get("tree_rst_fix0")))
print("  d1 で止めた (問題, 種, 回):", sorted((k[0], k[2], r["cnt"]["trnofix_stop"]) for k, r in R.items() if k[1] == "d1" and r.get("cnt", {}).get("trnofix_stop")))
print("\n== clique fallback の試し (秒の合計の大きい順、base / c3)")
for p in probs:
    for s in seeds:
        a, b = R.get((p, "base", s)), R.get((p, "c3", s))
        if a and b and (a["cnt"]["clqfb_secs"] > 0.5 or b["cnt"]["clqfb_capped"] or a["cnt"]["clqfb_used"] != b["cnt"]["clqfb_used"]):
            print(f"  {p} 種 {s}: base {a['cnt']['clqfb_secs']:.2f}s used {a['cnt']['clqfb_used']} / c3 {b['cnt']['clqfb_secs']:.2f}s used {b['cnt']['clqfb_used']} 省き・止め {b['cnt']['clqfb_capped']}")
H = load("H")
if H:
    print("\n== H (600 秒、種 1): 問題 | base (状態, 秒, 目的, 下界) | c3d1 | cnt の差")
    for p in sorted({p for p, c, s in H}):
        a, b = H.get((p, "base", 1)), H.get((p, "c3d1", 1))
        f = lambda r: "-" if not r else f"{r['result'].get('status')} {t(r):.0f}s {r['result'].get('objective')} / {r['result'].get('bound')}"
        print(f"  {p} | {f(a)} | {f(b)} | base {a and a.get('cnt')} c3d1 {b and b.get('cnt')}")

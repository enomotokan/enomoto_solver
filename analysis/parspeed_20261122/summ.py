"""parspeed の集計: python summ.py [res_P_s*.jsonl ...] > summ_P.txt (引数なしなら res_P_s*.jsonl 全部)。
問題ごとに種 1〜3 を (問題, 種) の組として扱い、SGM (shift 1 秒、時間切れは上限 120 秒で数える) と問題単位のブートストラップ 95% CI を出す。
- 並列の速さ: t4/t1 (暫定解なし)、t4c/t1c (最適値の打ち切り値あり = 暫定解の差を除いた並列の速さ)。
- 暫定解の得: t1c/t1、t4c/t4。
- 根と木の分け: 根の秒 (TRSTAT)、木の秒 (全体 - 根)。木の秒の比は両方の回が木に入った組だけ。ノードの比 (4 スレッドは全スレッドの和) も。
- 根で終わった回 (TRSTAT なし) は数だけ別に出す。"""
import json, glob, math, random, sys, collections
files = sys.argv[1:] or sorted(glob.glob("res_P_s*.jsonl"))
R = {}
for f in files:
    for l in open(f):
        if l.strip():
            r = json.loads(l); R[(r["problem"], r["cfg"], r["seed"])] = r
TL = 120.0
CF = ["t1", "t4", "t1c", "t4c"]
def solved(r): return str(r["result"].get("status")).lower() in ("optimal", "infeasible")
def secs(r): return min(r["result"]["time"], TL) if solved(r) else TL
def root(r): return r["split"]["root_secs"] if r["split"]["tree_started"] else None
def tree(r): return secs(r) - root(r) if r["split"]["tree_started"] else None
def nodes(r): return r["result"].get("nodes") or 0
probs = sorted({p for p, c, s in R})
pairs = sorted({(p, s) for p, c, s in R if all((p, c2, s) in R for c2 in CF)})
print(f"ファイル {len(files)}、行 {len(R)}、4 設定そろった (問題, 種) {len(pairs)} 組 ({len({p for p, s in pairs})} 問)")
st = collections.Counter((c, str(r["result"].get("status"))) for (p, c, s), r in R.items())
print("状態:", dict(sorted(st.items())))
rootonly = collections.Counter(c for (p, c, s), r in R.items() if not r["split"]["tree_started"] and solved(r))
print("根で終わった回 (木に入らず解けた):", dict(rootonly))
# 最適値との食い違い
bad = [(p, c, s, r["result"]["objective"], r["opt"]) for (p, c, s), r in R.items()
       if str(r["result"].get("status")) == "optimal" and r["result"]["objective"] is not None and abs(r["result"]["objective"] - r["opt"]) > 1e-6 * max(1, abs(r["opt"]))]
print("optimal で最適値 (solu) と 1e-6 を超えて違う回:", bad or "なし")
def sgm_ratio(num, den, sel, shift=1.0):
    """組 (p, s) のうち sel が真のものについて、問題ごとに種の平均の log 比 -> 問題の平均 -> exp。CI は問題単位のブートストラップ"""
    byp = collections.defaultdict(list)
    for p, s in pairs:
        a, b = num(p, s), den(p, s)
        if a is None or b is None or not sel(p, s): continue
        byp[p].append(math.log((a + shift) / (b + shift)))
    ps = sorted(byp)
    if not ps: return None
    v = [sum(byp[p]) / len(byp[p]) for p in ps]
    est = math.exp(sum(v) / len(v))
    rnd = random.Random(1); bs = []
    for _ in range(2000):
        x = [v[rnd.randrange(len(v))] for _ in v]; bs.append(math.exp(sum(x) / len(x)))
    bs.sort()
    return est, bs[50], bs[1949], len(ps), sum(len(byp[p]) for p in ps)
g = lambda c, f: (lambda p, s: f(R[(p, c, s)]))
allp = lambda p, s: True
both_tree = lambda a, b: (lambda p, s: R[(p, a, s)]["split"]["tree_started"] and R[(p, b, s)]["split"]["tree_started"])
def show(name, x):
    print(f"  {name}: " + ("なし" if x is None else f"{x[0]:.3f} [{x[1]:.3f}, {x[2]:.3f}] ({x[3]} 問・{x[4]} 組)"))
print("\n## 全体の秒 (SGM、shift 1 秒、時間切れは 120 秒)")
show("t4/t1 (並列の速さ、暫定解なし)", sgm_ratio(g("t4", secs), g("t1", secs), allp))
show("t4c/t1c (並列の速さ、最適値の打ち切り値あり)", sgm_ratio(g("t4c", secs), g("t1c", secs), allp))
show("t1c/t1 (1 スレッドで暫定解を与えた得)", sgm_ratio(g("t1c", secs), g("t1", secs), allp))
show("t4c/t4 (4 スレッドで暫定解を与えた得)", sgm_ratio(g("t4c", secs), g("t4", secs), allp))
print("\n## 根と木 (両方の回が木に入った組だけ)")
show("根の秒 t4/t1 (shift 0.1)", sgm_ratio(g("t4", root), g("t1", root), both_tree("t4", "t1"), 0.1))
show("木の秒 t4/t1", sgm_ratio(g("t4", tree), g("t1", tree), both_tree("t4", "t1")))
show("木の秒 t4c/t1c", sgm_ratio(g("t4c", tree), g("t1c", tree), both_tree("t4c", "t1c")))
show("ノード t4/t1 (shift 1)", sgm_ratio(g("t4", nodes), g("t1", nodes), both_tree("t4", "t1")))
show("ノード t4c/t1c (shift 1)", sgm_ratio(g("t4c", nodes), g("t1c", nodes), both_tree("t4c", "t1c")))
show("ノード t1c/t1 (shift 1)", sgm_ratio(g("t1c", nodes), g("t1", nodes), both_tree("t1c", "t1")))
show("ノード t4c/t4 (shift 1)", sgm_ratio(g("t4c", nodes), g("t4", nodes), both_tree("t4c", "t4")))
def solved_frac(c): return sum(solved(R[(p, c, s)]) for p, s in pairs), len(pairs)
print("\n解けた組:", {c: "%d/%d" % solved_frac(c) for c in CF})
print("\n## 問題ごと (種 1〜3 の中央値。秒 (時間切れは 120 と書き * をつける)・ノード・根の秒。比は中央値どうし)")
print("| 問題 | t1 秒 | t4 秒 | t1c 秒 | t4c 秒 | t1 ノード | t4 ノード | t1c ノード | t4c ノード | 根 t1 | 根 t4 | t4/t1 | t4c/t1c | t1c/t1 | t4c/t4 | 木 t4/t1 | ノード t4/t1 |")
print("|" + "---|" * 17)
med = lambda xs: sorted(xs)[len(xs) // 2] if xs else None
for p in sorted({p for p, s in pairs}):
    ss = [s for q, s in pairs if q == p]
    def m(c, f): return med([v for v in (f(R[(p, c, s)]) for s in ss) if v is not None])
    T = {c: m(c, secs) for c in CF}; N = {c: m(c, nodes) for c in CF}
    tl = {c: any(not solved(R[(p, c, s)]) for s in ss) for c in CF}
    rt = {c: m(c, root) for c in ("t1", "t4")}; tr = {c: m(c, tree) for c in ("t1", "t4")}
    fs = lambda c: f"{T[c]:.1f}" + ("*" if tl[c] else "")
    fr = lambda a, b: f"{(a + 1) / (b + 1):.2f}" if a is not None and b is not None else "-"
    fo = lambda x: "-" if x is None else f"{x:.1f}"
    print(f"| {p} | {fs('t1')} | {fs('t4')} | {fs('t1c')} | {fs('t4c')} | {N['t1']} | {N['t4']} | {N['t1c']} | {N['t4c']} | {fo(rt['t1'])} | {fo(rt['t4'])} | "
          f"{fr(T['t4'], T['t1'])} | {fr(T['t4c'], T['t1c'])} | {fr(T['t1c'], T['t1'])} | {fr(T['t4c'], T['t4'])} | {fr(tr['t4'], tr['t1'])} | {fr(N['t4'], N['t1'])} |")
print("\n(* は種のどれかが時間切れ。表の比は shift 1 の (a+1)/(b+1)。)")

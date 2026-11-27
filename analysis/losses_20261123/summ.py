"""負けている問題の測り直し (analysis/losses_20261123) の集計 (analysis/holdout_20261123/summ.py の写し): python summ.py [res_s*.jsonl ...] [--tl 600] [--so .so の sha1 の先頭]
- 状態の内訳、解けた数 (optimal・infeasible)、最大 RSS (設定ごと)。
- 食い違い: 同じ (問題, 種) の設定どうしの最適値の不一致・最適と実行不能の衝突、MIPLIB の既知の値 (solu.json) との不一致
  (=opt= と違う最適値、=inf= なのに解あり、既知の値より良い「最適」や実行可能解、=opt= の問題で「実行不能」)。
- 比べ: ours_t1 / scip、ours_t1 / highs_t1、ours_t4 / highs_t4 の SGM (shift 1 秒、時間切れ・解けない回は上限で数える)、問題単位のブートストラップの
  95% 区間、片方だけ解けた数、2 倍以上負けた問題 (と勝った問題) の一覧。比は「前 / 後」の時間の比で、1 より小さければ当方が速い。
"""
import collections, glob, json, math, random, sys
from pathlib import Path

D = Path(__file__).resolve().parent
args = [x for x in sys.argv[1:]]
TL = None
SO = None
if "--so" in args:
    i = args.index("--so"); SO = args[i + 1]; del args[i:i + 2]
if "--tl" in args:
    i = args.index("--tl"); TL = float(args[i + 1]); del args[i:i + 2]
files = args or [str(D / "res_s*.jsonl")]
rows = [json.loads(l) for f in files for g in glob.glob(f) for l in open(g, encoding="utf-8") if l.strip()]
if TL is not None:
    rows = [r for r in rows if r.get("tl") == TL]
if SO is not None:
    rows = [r for r in rows if not r["cfg"].startswith("ours") or ((r.get("versions") or {}).get("ours_so_sha1") or "").startswith(SO)]
# 同じ (問題, 設定, 種, 上限) が何行もあれば最後の行を使う
R = {}
for r in rows:
    R[(r["problem"], r["cfg"], r["seed"], r.get("tl"))] = r
rows = list(R.values())
tls = sorted({r.get("tl") for r in rows})
if len(tls) > 1:
    sys.exit(f"上限秒の違う行が混ざっている {tls}: --tl で選ぶ")
SOLU = json.loads((D / "solu.json").read_text(encoding="utf-8"))
CF = ["ours_t1", "ours_t4", "scip", "highs_t1", "highs_t4"]
cfgs = [c for c in CF if any(r["cfg"] == c for r in rows)]


def st(r):
    s = str(r["result"].get("status") or "").lower()
    return {"time limit reached": "time_limit", "primal infeasible or unbounded": "infeasible_or_unbounded"}.get(s, s)


def solved(r): return st(r) in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) and r["result"].get("time") is not None else r["tl"]
def obj(r):
    """目的値 (有限でなければ None。HiGHS は実行不能のときも目的値 inf を返す)"""
    v = r["result"].get("objective")
    return v if isinstance(v, (int, float)) and math.isfinite(v) else None
def same(a, b): return abs(a - b) <= 1e-6 * max(1.0, abs(a), abs(b))


K = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
probs = sorted({r["problem"] for r in rows}); seeds = sorted({r["seed"] for r in rows})
print(f"行数 {len(rows)}、問題 {len(probs)}、種 {seeds}、上限 {tls} 秒、設定 {cfgs}")
vers = collections.Counter(json.dumps(r.get("versions"), sort_keys=True) for r in rows)
print("版:", dict(vers))
# 当方の .so の sha1 ごとの行の数 (.so は作るたびに変わる。README 3 節。同じ作り方なら動きは同じ)
so_n = collections.Counter(((r.get("versions") or {}).get("ours_so_sha1") or "")[:8] for r in rows if r["cfg"].startswith("ours"))
print("当方の .so の sha1 ごとの行の数:", dict(so_n), "(--so 先頭 8 文字 で当方の行をその .so だけにできる)")
print("\n== 状態の内訳")
for c in cfgs:
    print(f"  {c}: {dict(collections.Counter(st(r) for r in rows if r['cfg'] == c))}")
crash = [(r["problem"], r["cfg"], r["seed"], st(r)) for r in rows if st(r) in ("", "crash", "killed", "error", "fetch_error")]
print(f"  crash・killed・取得失敗 {len(crash)}: {crash}")
print("\n== 解けた数 (全設定がそろった (問題, 種) で)")
full = [(p, s) for p in probs for s in seeds if all((p, c, s) in K for c in cfgs)]
print(f"  {len(full)} 組: " + ", ".join(f"{c} {sum(solved(K[(p, c, s)]) for p, s in full)}" for c in cfgs))
print("\n== 最大 RSS")
for c in cfgs:
    m = max((r for r in rows if r["cfg"] == c), key=lambda r: r.get("max_rss_mb") or 0)
    print(f"  {c}: {m.get('max_rss_mb', 0)} MB ({m['problem']})")

print("\n== 食い違い")
bad = []
for p in probs:
    sol = SOLU.get(p)
    for s in seeds:
        rs = [(c, K[(p, c, s)]) for c in cfgs if (p, c, s) in K]
        opts = [(c, obj(r)) for c, r in rs if st(r) == "optimal" and obj(r) is not None]
        if opts and not all(same(o, opts[0][1]) for _, o in opts):
            bad.append((p, s, "最適値の不一致", opts))
        sts = {st(r) for _, r in rs}
        if "optimal" in sts and "infeasible" in sts:
            bad.append((p, s, "最適と実行不能", [(c, st(r)) for c, r in rs]))
        if not sol:
            continue
        for c, r in rs:
            o = obj(r)
            if sol["kind"] == "inf" and (o is not None or st(r) == "optimal"):
                bad.append((p, s, c, f"既知は実行不能なのに {st(r)} {o}"))
            if sol["kind"] == "opt" and st(r) == "infeasible":
                bad.append((p, s, c, f"既知の最適値 {sol['value']} があるのに実行不能"))
            if sol["kind"] == "opt" and st(r) == "optimal" and o is not None and not same(o, sol["value"]):
                bad.append((p, s, c, f"最適値 {o} が既知の {sol['value']} と違う"))
            if sol["kind"] in ("opt", "best") and o is not None and sol["value"] is not None and o < sol["value"] - 1e-6 * max(1.0, abs(sol["value"])):
                # MIPLIB の問題はどれも最小化
                bad.append((p, s, c, f"解 {o} が既知の値 {sol['value']} より良い"))
print(f"  {len(bad)} 件")
for b in bad:
    print("  ", b)


def cmp(ca, cb):
    """ca (当方) / cb (相手) の時間の比"""
    pairs = [(p, s) for p in probs for s in seeds if (p, ca, s) in K and (p, cb, s) in K]
    if not pairs:
        return
    logr = collections.defaultdict(list)
    for p, s in pairs:
        a, b = K[(p, ca, s)], K[(p, cb, s)]
        if solved(a) or solved(b):
            logr[p].append(math.log((t(a) + 1) / (t(b) + 1)))
    ps = sorted(logr)
    na = sum(solved(K[(p, ca, s)]) for p, s in pairs); nb = sum(solved(K[(p, cb, s)]) for p, s in pairs)
    wa = sum(solved(K[(p, ca, s)]) and not solved(K[(p, cb, s)]) for p, s in pairs)
    wb = sum(solved(K[(p, cb, s)]) and not solved(K[(p, ca, s)]) for p, s in pairs)
    print(f"\n== {ca} / {cb}: {len(pairs)} 組、解けた {ca} {na}・{cb} {nb} ({ca} だけ {wa}、{cb} だけ {wb})")
    if not ps:
        print("  どちらかが解けた問題がない")
        return

    def sgm(q):
        v = [x for p in q for x in logr[p]]
        return math.exp(sum(v) / len(v))
    random.seed(0)
    bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000))
    print(f"  SGM の比 (どちらかが解けた {len(ps)} 問、shift 1 秒): {sgm(ps):.3f} (95% [{bs[100]:.3f}, {bs[3899]:.3f}])  1 より小さければ {ca} が速い")
    g = {p: math.exp(sum(v) / len(v)) for p, v in logr.items()}
    lose = sorted((p for p in ps if g[p] >= 2), key=lambda p: -g[p])
    win = sorted((p for p in ps if g[p] <= 0.5), key=lambda p: g[p])

    def desc(p):
        return ", ".join(f"種 {s}: {st(K[(p, ca, s)])} {t(K[(p, ca, s)]):.1f} 秒 / {st(K[(p, cb, s)])} {t(K[(p, cb, s)]):.1f} 秒" for s in seeds if (p, ca, s) in K and (p, cb, s) in K)
    print(f"  2 倍以上負けた問題 {len(lose)}:")
    for p in lose:
        print(f"    {p}: {g[p]:.2f} ({desc(p)})")
    print(f"  2 倍以上勝った問題 {len(win)}:")
    for p in win:
        print(f"    {p}: {g[p]:.2f} ({desc(p)})")


for ca, cb in (("ours_t1", "scip"), ("ours_t1", "highs_t1"), ("ours_t4", "highs_t4")):
    if ca in cfgs and cb in cfgs:
        cmp(ca, cb)

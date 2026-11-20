"""今の既定 (def 4 スレッド・def_t1 1 スレッド) と SCIP (scip、逐次、11/17 の行を使い回す) の比較 (2026-11-20 の測り直し) (M: Mittelmann 186 問、P: MIPLIB の 58 問)。
python summ.py M > summ_M.txt、python summ.py P > summ_P.txt。設定は cfg.py。
SGM は shift 1 秒、時間切れは制限時間で数える。CI は問題単位のブートストラップ (vshighs_20261117/summ.py と同じ)。
参考: analysis/vshighs_20261117 の HiGHS 1.15.1 (4 スレッド) と当方 def の結果を、同じ (問題, 種) で並べる (機械・時期が違うので参考)。"""
import json, glob, math, random, collections, sys
from pathlib import Path
D = Path(__file__).resolve().parent
SET = sys.argv[1] if len(sys.argv) > 1 else "P"
rows = [json.loads(l) for f in sorted(glob.glob(str(D / f"res_{SET}_s*.jsonl"))) for l in open(f) if l.strip()]
# SCIP の行は 11/17 (analysis/vsscip_20261117、同じ機械・同じ種) を使い回す。11/17 の当方の def・def_t1 は「@1117」を付けて前の版として並べる
OLD = D.parent / "vsscip_20261117"
for f in sorted(glob.glob(str(OLD / f"res_{SET}_s*.jsonl"))):
    for l in open(f):
        if l.strip():
            r = json.loads(l)
            if r["cfg"] != "scip": r["cfg"] += "@1117"
            rows.append(r)
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
CF = ["def", "def_t1", "scip"]
def st(r): return str(r["result"].get("status") or "").lower()  # HiGHS は "Optimal"・"Time limit reached"、SCIP は worker_scip で揃えた名前
def solved(r): return st(r) in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
probs = sorted({p for p, c, s in R}); seeds = sorted({s for p, c, s in R})
crash = [(r["problem"], r["cfg"], r["seed"], st(r)) for r in rows if "error" in r["result"] or st(r) in ("", "crash", "error", "fetch_error")]
print(f"{SET}: 行数 {len(rows)}、問題 {len(probs)}、種 {seeds}、crash {len(crash)} {crash}")
vers = {r["result"].get("scip_version") for r in rows if r["cfg"] == "scip"} - {None}
print("SCIP の版:", sorted(vers), "pyscipopt:", sorted({r["result"].get("pyscipopt_version") for r in rows if r["cfg"] == "scip"} - {None}))
print("状態:", {c: dict(collections.Counter(st(r) for r in rows if r["cfg"] == c)) for c in CF + ["def@1117", "def_t1@1117"]})
# 3 者そろった (問題, 種) だけで数える
full = [(p, s) for p in probs for s in seeds if all((p, c, s) in R for c in CF)]
print("解けた数 (3 者そろった", len(full), "組):", {c: sum(solved(R[(p, c, s)]) for p, s in full) for c in CF})
# 参考の HiGHS (vshighs_20261117。M は res_M (種 1・2) と res_A (種 1)、P と L は res_P)
VH = D.parent / "vshighs_20261117"
H = {}
for S in {"M": ("M", "A"), "L": ("P",)}.get(SET, (SET,)):  # L (手元の確認) は P の結果と並べる
    for f in glob.glob(str(VH / f"res_{S}_s*.jsonl")):
        for l in open(f):
            if l.strip():
                r = json.loads(l); H[(r["problem"], r["cfg"] + "@vsh", r["seed"])] = r
def same(a, b): return abs(a - b) <= 1e-6 * max(1, abs(a), abs(b))
bad = []
for p, s in full:
    rs = [(c, R[(p, c, s)]) for c in CF] + [(c, H[(p, c, s)]) for c in ("def@vsh", "highs@vsh") if (p, c, s) in H]
    objs = [(c, r["result"].get("objective")) for c, r in rs if st(r) == "optimal" and r["result"].get("objective") is not None]
    if objs and not all(same(o, objs[0][1]) for _, o in objs): bad.append((p, s, objs))
    if {st(r) for _, r in rs} >= {"optimal", "infeasible"}: bad.append((p, s, "opt/inf", [(c, st(r)) for c, r in rs]))
print("食い違い (参考の HiGHS・vshighs の def も含む)", len(bad))
for b in bad: print(" ", b)
for c in CF:
    rr = [r for r in rows if r["cfg"] == c]
    m = max(rr, key=lambda r: r.get("max_rss_mb") or 0, default=None)
    print(c, "最大 RSS", (m or {}).get("max_rss_mb", 0), "MB", f"({m['problem']})" if m else "")
def cmp(ca, cb):
    logr = collections.defaultdict(list)
    full = [(p, s) for p, s in globals()["full"] if (p, ca, s) in R and (p, cb, s) in R]
    for p, s in full:
        a, b = R[(p, ca, s)], R[(p, cb, s)]
        if solved(a) or solved(b): logr[p].append(math.log((t(b) + 1) / (t(a) + 1)))
    ps = list(logr)
    def sgm(q):
        v = [x for p in q for x in logr[p]]; return math.exp(sum(v) / len(v)) if v else 1.0
    random.seed(0); bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000))
    print(f"\n== {cb} / {ca} (どちらかが解けた {len(ps)} 問、負なら {cb} が速い): {sgm(ps)-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}]")
    w = sum(solved(R[(p, cb, s)]) and not solved(R[(p, ca, s)]) for p, s in full); l_ = sum(solved(R[(p, ca, s)]) and not solved(R[(p, cb, s)]) for p, s in full)
    print(f"  {cb} だけ解けた {w} 組、{ca} だけ解けた {l_} 組")
    for p in sorted(ps, key=lambda q: -sum(logr[q]) / len(logr[q])):
        v = logr[p]; g = math.exp(sum(v) / len(v))
        tag = " 全種で遅" if len(v) >= 2 and all(x > math.log(1.25) for x in v) else (" 全種で速" if len(v) >= 2 and all(x < math.log(0.8) for x in v) else "")
        if g > 2 or g < 0.5: print(f"  {p}: {g:.2f} {[round(math.exp(x), 2) for x in v]}{tag}")
cmp("scip", "def")
cmp("scip", "def_t1")
cmp("def", "def_t1")
# 前の版 (11/17) との比べ (同じ機械・同じ種、負なら今の版が速い)
cmp("def@1117", "def")
cmp("def_t1@1117", "def_t1")
# 参考の表: 3 者と vshighs の def・highs がそろった (問題, 種) で、解けた数と SGM (shift 1、時間切れは上限)
ref = [(p, s) for p, s in full if all((p, c, s) in H for c in ("def@vsh", "highs@vsh"))]
def g(c, q):
    v = [math.log(t(R.get((p, c, s)) or H[(p, c, s)]) + 1) for p, s in q]; return math.exp(sum(v) / len(v)) - 1 if v else float("nan")
print(f"\n== 参考の表 ({len(ref)} 組。@vsh は analysis/vshighs_20261117 の回で、機械・時期が違うので参考。highs は 4 スレッド)")
print("| 設定 | 解けた数 | SGM (秒、shift 1) | scip との比 |")
print("|---|---|---|---|")
for c in CF + ["def@vsh", "highs@vsh"]:
    n = sum(solved(R.get((p, c, s)) or H[(p, c, s)]) for p, s in ref)
    print(f"| {c} | {n} | {g(c, ref):.1f} | {(g(c, ref) + 1) / (g('scip', ref) + 1):.2f} |")

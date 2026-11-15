"""cand1114 の Mittelmann の絞った組 (plan.py の 48 問、HiGHS は同じ分担・同じ機械の種 1 の回) × 種 1・2 (600 秒) の対の比較と HiGHS との比べ
(../cand1113_20261112/summM.py の流用)。最後に cand1113 の M の結果と突き合わせる (束の効きが残ったか・遅い 5 問が直ったか)。
python summM.py [res のあるディレクトリ] > summM.txt
SGM は shift 10 秒 (Mittelmann の慣例、cmp600i.py と同じ)、時間切れは 600 秒。CI は問題単位のブートストラップ。
誤答 = 実行不能と返したが、同じ問題のほかの回 (def・束・HiGHS のどれか) が最適解を出した回。解けていない扱いにする。"""
import json, glob, math, random, collections, sys
from pathlib import Path
D = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent
C13 = Path(__file__).resolve().parents[1] / "cand1113_20261112"
TL = 600.0
rows = [json.loads(l) for f in sorted(glob.glob(str(D / "res_M_s*.jsonl"))) for l in open(f)]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
H = {}
for r in rows:
    if r["cfg"] == "highs": H[r["problem"]] = r
def st(r): return str(r["result"].get("status", "")).lower()
opt = {r["problem"] for r in rows if st(r).startswith("optimal")}
WRONG = {(r["problem"], r["cfg"], r["seed"]) for r in rows if st(r).startswith("infeasible") and r["problem"] in opt}
def solved(r): return (r["problem"], r["cfg"], r["seed"]) not in WRONG and (st(r).startswith("optimal") or st(r).startswith("infeasible")) and (r["result"].get("time") or 1e9) <= TL + 5
def t(r): return min(r["result"]["time"], TL) if solved(r) else TL
hs = {p for p, r in H.items() if solved(r)}
probs = sorted({p for p, c, s in R if c != "highs"}); seeds = [1, 2]
crash = sum(1 for r in rows if st(r) in ("crash", "error", "") or "error" in r["result"])
ns = collections.Counter(); ho = collections.Counter(); eo = collections.Counter(); bad = []
logr = collections.defaultdict(list); pair = 0
for p in probs:
    for s in seeds:
        a, b = R.get((p, "def", s)), R.get((p, "cand1114", s))
        if not a or not b: continue
        pair += 1
        for c, x in (("def", a), ("cand1114", b)):
            ns[c] += solved(x)
            if p in hs and not solved(x): ho[c] += 1
            if p not in hs and solved(x): eo[c] += 1
        oa, ob = a["result"].get("objective"), b["result"].get("objective")
        if st(a).startswith("optimal") and st(b).startswith("optimal") and oa is not None and ob is not None and abs(oa - ob) > 1e-6 * max(1, abs(oa)):
            bad.append((p, s, oa, ob))
        for x, y in ((a, b), (b, a)):
            ba, ob2 = x["result"].get("bound"), y["result"].get("objective")
            if solved(x) is False and ba is not None and ob2 is not None and solved(y) and math.isfinite(ba) and ba > ob2 + 1e-6 * max(1, abs(ob2)):
                bad.append((p, s, "bound>opt", ba, ob2))
        if solved(a) or solved(b):
            logr[p].append(math.log((t(b) + 10) / (t(a) + 10)))
def sgm_all(c): return math.exp(sum(math.log(t(R[(p, c, s)]) + 10) for p in probs for s in seeds if (p, c, s) in R) / pair) - 10
def ratio(ps):
    v = [x for p in ps for x in logr[p]]
    return math.exp(sum(v) / len(v))
ps = list(logr); random.seed(0)
bs = sorted(ratio([random.choice(ps) for _ in ps]) for _ in range(4000))
print(f"行数 {len(rows)} (対 {pair})、crash {crash} 、食い違い {len(bad)} {bad}、誤答 {len(WRONG)} {sorted(WRONG)} (解けていない扱い)")
print(f"解けた ({2 * len(probs)} 回): def {ns['def']} / cand1114 {ns['cand1114']}")
print(f"SGM (shift 10、全 {2 * len(probs)} 回): def {sgm_all('def'):.2f} / cand1114 {sgm_all('cand1114'):.2f}")
print(f"対の比 (どちらかが解けた {len(ps)} 問): {ratio(ps)-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}]")
print(f"HiGHS だけが解く (延べ): def {ho['def']} / cand1114 {ho['cand1114']}、ENOMOTO だけが解く (延べ): def {eo['def']} / cand1114 {eo['cand1114']}")
print("片方だけ解けた (問題 種: def 秒 / cand1114 秒):")
for p in probs:
    for s in seeds:
        a, b = R.get((p, "def", s)), R.get((p, "cand1114", s))
        if a and b and solved(a) != solved(b):
            print(f"  {p} {s}: {t(a):.0f} / {t(b):.0f}")
print("2 種とも 1.25 倍以上遅い / 0.8 倍以下に速い:")
for p in ps:
    v = logr[p]
    if len(v) == 2 and all(x > math.log(1.25) for x in v): print("  遅", p, [round(math.exp(x), 2) for x in v])
    if len(v) == 2 and all(x < math.log(0.8) for x in v): print("  速", p, [round(math.exp(x), 2) for x in v])
for c in ("def", "cand1114"):
    m = [r["max_rss_mb"] for r in rows if r["cfg"] == c and r.get("max_rss_mb")]
    print(c, "最大 RSS", max(m), "MB、5 GB 超", sum(x > 5000 for x in m))

# HiGHS (種 1、同じ分担・同じ機械) との比べ
hp = [p for p in probs if p in H and (p, "def", 1) in R and (p, "cand1114", 1) in R]
def tl(r, L): return solved(r) and t(r) <= L
for L in (60, 600):
    print(f"{L} 秒までに解けた (種 1、{len(hp)} 問): HiGHS {sum(tl(H[p], L) for p in hp)} / def {sum(tl(R[(p,'def',1)], L) for p in hp)} / cand1114 {sum(tl(R[(p,'cand1114',1)], L) for p in hp)}")
    for c in ("def", "cand1114"):
        v = [math.log((min(t(R[(p, c, 1)]), L) + 10) / (min(t(H[p]), L) + 10)) for p in hp if tl(H[p], L) or tl(R[(p, c, 1)], L)]
        print(f"  {c} / HiGHS の時間の比 (SGM shift 10、{L} 秒で打ち切り、どちらかが解けた {len(v)} 問): {math.exp(sum(v)/len(v)):.2f}")
print("HiGHS だけが解く (種 1、cand1114 は 600 秒で解けない): ", [(p, round(t(H[p]))) for p in hp if solved(H[p]) and not solved(R[(p, 'cand1114', 1)])])
print("cand1114 だけが解く (種 1、HiGHS は解けない): ", [(p, round(t(R[(p, 'cand1114', 1)]))) for p in hp if solved(R[(p, 'cand1114', 1)]) and not solved(H[p])])
print("両方解けて cand1114 が HiGHS の 2 倍以上遅い (種 1): ", [(p, round(t(R[(p, 'cand1114', 1)])), round(t(H[p]))) for p in hp if solved(H[p]) and solved(R[(p, 'cand1114', 1)]) and t(R[(p, 'cand1114', 1)]) + 10 > 2 * (t(H[p]) + 10)])

# cand1113 の M (同じ問題、同じ作りの回) との突き合わせ。比 = 束 / def (shift 10)、種 1・2。* は解けなかった回のある側
rows13 = [json.loads(l) for f in sorted(glob.glob(str(C13 / "res_M_s*.jsonl"))) for l in open(f)]
R13 = {(r["problem"], r["cfg"], r["seed"]): r for r in rows13}
W13 = {("unitcal_7", "cand1113", 1), ("unitcal_7", "cand1113", 2)}
def solved13(r): return (r["problem"], r["cfg"], r["seed"]) not in W13 and (st(r).startswith("optimal") or st(r).startswith("infeasible")) and (r["result"].get("time") or 1e9) <= TL + 5
def t13(r): return min(r["result"]["time"], TL) if solved13(r) else TL
def cell(RR, sv, tt, p, c):
    out = []
    for s in seeds:
        a, b = RR.get((p, "def", s)), RR.get((p, c, s))
        if not a or not b: out.append("-"); continue
        mk = lambda x: "" if sv(x) else ("*" if x["result"].get("objective") is not None else "-")
        out.append(f"{tt(a):.0f}{mk(a)}/{tt(b):.0f}{mk(b)}")
    return " ".join(out)
def rat(RR, tt, p, c):
    v = [math.log((tt(RR[(p, c, s)]) + 10) / (tt(RR[(p, "def", s)]) + 10)) for s in seeds if (p, c, s) in RR and (p, "def", s) in RR]
    return math.exp(sum(v) / len(v)) if v else float("nan")
GROUPS = [  # plan.py と同じ組
    ("cand1113 で 2 種とも遅かった 5 問 (原因の項目、cand1114 では外した)", [("enlight_hard", "HNFPRE=0"), ("neos-1122047", "IPMRC=utw"), ("cbs-cta", "CLQPRB"),
                                                                    ("roll3000", "CLQPRB"), ("neos-960392", "SUBFIX=0")]),
    ("cand1113 の誤答", [("unitcal_7", "直し 40e80904")]),
    ("HiGHS に負けている 12 問 (analysis/highsloss_20261113)", [(p, "") for p in ["hypothyroid-k1", "mzzv11", "mzzv42z", "neos-3216931-puriri", "neos-3627168-kasai",
        "neos-873061", "neos-933966", "net12", "rococoC10-001000", "satellites2-60-fs", "sct2", "uccase12"]]),
    ("cand1113 だけが片方の種で解いた問題・2 種とも速かった問題 (効きが残るか)", [(p, "") for p in dict.fromkeys(["CMS750_4", "csched008", "fast0507", "fiball",
        "gen-ip054", "hypothyroid-k1", "istanbul-no-cutoff", "mcsched", "neos-1456979", "neos-3627168-kasai", "neos-4722843-widden", "neos-662469", "neos-787933",
        "peg-solitaire-a3", "pg5_34", "ran14x18-disj-8", "rocI-4-11", "snp-02-004-104", "supportcase33", "supportcase40", "tr12-30", "traininstance6",
        "fastxgemm-n2r6s0t2", "h80x6320d", "irp", "mas74", "mas76", "neos-2978193-inde", "neos-4738912-atrato", "neos5", "tbfp-network", "timtab1"])]),
]
print("\ncand1113 の M との突き合わせ (秒は def/束、種 1 種 2。* は解けなかった回、- は解けず暫定解もない回。比は束 / def の幾何平均 (shift 10)):")
cnt = collections.Counter()
for title, items in GROUPS:
    print(f"\n## {title}\n| 問題 | 備考 | cand1113 の回 (def/cand1113) | 比 | cand1114 の回 (def/cand1114) | 比 | 判定 |\n|---|---|---|---|---|---|---|")
    for p, note in items:
        r13, r14 = rat(R13, t13, p, "cand1113"), rat(R, t, p, "cand1114")
        n13 = sum(solved13(R13[(p, "cand1113", s)]) - solved13(R13[(p, "def", s)]) for s in seeds if (p, "cand1113", s) in R13 and (p, "def", s) in R13)
        n14 = sum(solved(R[(p, "cand1114", s)]) - solved(R[(p, "def", s)]) for s in seeds if (p, "cand1114", s) in R and (p, "def", s) in R)
        if math.isnan(r14): j = "未"
        elif r13 >= 1.25: j = "直った" if r14 < 1.25 else "遅いまま"
        elif r13 <= 0.8 or n13 > 0: j = "残った" if (r14 <= 0.8 or n14 > 0) else "消えた"
        else: j = "差なし" if 0.8 < r14 < 1.25 else ("遅い" if r14 >= 1.25 else "速い")
        cnt[j] += 1
        print(f"| {p} | {note} | {cell(R13, solved13, t13, p, 'cand1113')} | {r13:.2f} | {cell(R, solved, t, p, 'cand1114')} | {r14:.2f} | {j} |")
print("\n判定の数:", dict(cnt))

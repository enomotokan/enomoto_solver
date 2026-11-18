"""gate2 候補 4 (nodelp2s、slow2_20261117 の写し): 既定 (def) と試作 (wst) の対の比較 (M: Mittelmann 48 問、P: MIPLIB の 58 問)。python summ.py P (または M) [wst|sw] > summ_P.txt
設定は cfg.py。
SGM は shift 1 秒、時間切れは制限時間で数える。CI は問題単位のブートストラップ (summP.py と同じ)。"""
import json, glob, math, random, collections, sys
from pathlib import Path
D = Path(__file__).resolve().parent
SET = sys.argv[1] if len(sys.argv) > 1 else "P"
ALT = sys.argv[2] if len(sys.argv) > 2 else "xs"  # 既定と比べる設定 
rows = [json.loads(l) for S in (("M", "A") if SET == "MA" else (SET,)) for f in sorted(glob.glob(str(D / f"res_{S}_s*.jsonl"))) for l in open(f) if l.strip()]  # MA = M (48 問 × 2 種) と残りの A (138 問 × 種 1) を合わせた Mittelmann 186 問
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
CF = ["def", ALT]
def st(r): return str(r["result"].get("status") or "").lower()  # 当方は小文字
def solved(r): return st(r) in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
probs = sorted({p for p, c, s in R}); seeds = sorted({s for p, c, s in R})
crash = sum(1 for r in rows if "error" in r["result"] or st(r) in (None, "crash", "error", "fetch_error"))
print(f"{SET}: 行数 {len(rows)}、問題 {len(probs)}、種 {seeds}、crash {crash}")
# 2 者そろった (問題, 種) だけで数える
full = [(p, s) for p in probs for s in seeds if all((p, c, s) in R for c in CF)]
print("解けた数 (2 者そろった", len(full), "組):", {c: sum(solved(R[(p, c, s)]) for p, s in full) for c in CF})
bad = []
for p, s in full:
    objs = [R[(p, c, s)]["result"].get("objective") for c in CF if st(R[(p, c, s)]) == "optimal"]
    if objs and max(objs) - min(objs) > 1e-6 * max(1, abs(objs[0])): bad.append((p, s, objs))
    if {st(R[(p, c, s)]) for c in CF} >= {"optimal", "infeasible"}: bad.append((p, s, "opt/inf"))
print("食い違い", len(bad), bad)
for c in CF:
    print(c, "最大 RSS", max([(r.get("max_rss_mb") or 0) for r in rows if r["cfg"] == c] or [0]), "MB")
def cmp(ca, cb):
    logr = collections.defaultdict(list)
    for p, s in full:
        a, b = R[(p, ca, s)], R[(p, cb, s)]
        if solved(a) or solved(b): logr[p].append(math.log((t(b) + 1) / (t(a) + 1)))
    ps = list(logr)
    def sgm(q):
        v = [x for p in q for x in logr[p]]; return math.exp(sum(v) / len(v)) if v else 1.0
    random.seed(0); bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000))
    print(f"\n== {cb} / {ca} (どちらかが解けた {len(ps)} 問): {sgm(ps)-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}]")
    for p in sorted(ps, key=lambda q: -sum(logr[q]) / len(logr[q])):
        v = logr[p]; g = math.exp(sum(v) / len(v))
        tag = " 全種で遅" if len(v) >= 2 and all(x > math.log(1.25) for x in v) else (" 全種で速" if len(v) >= 2 and all(x < math.log(0.8) for x in v) else "")
        if g > 1.15 or g < 0.87: print(f"  {p}: {g:.2f} {[round(math.exp(x), 2) for x in v]}{tag}")
cmp("def", ALT)

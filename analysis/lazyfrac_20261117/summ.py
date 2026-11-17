"""LAZYFRAC の対の比べの集計: python summ.py P > summ_P.txt、python summ.py M > summ_M.txt (analysis/genlazy_20261117/summ.py の写し)。
各設定を def と比べる (ずらし幾何平均、shift 1 秒、時間切れは制限時間。95% 区間は問題単位のブートストラップ)。
後半は問題ごとの f (外せる非零の割合)・外した行と速さの比の並び (門が問題を分けたかを見る材料)。M は (問題, 種) ごとの秒・時間切れの下界も出す。"""
import json, glob, math, random, collections, sys
from pathlib import Path
D = Path(__file__).resolve().parent
SET = sys.argv[1] if len(sys.argv) > 1 else "P"
rows = [json.loads(l) for f in sorted(glob.glob(str(D / f"res_{SET}_s*.jsonl"))) for l in open(f) if l.strip()]
R = {(r["problem"], r["cfg"], r["seed"]): r for r in rows}
CF = ["def"] + sorted({c for _, c, _ in R} - {"def"})
def st(r): return str(r["result"].get("status") or "").lower()
def solved(r): return st(r) in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
probs = sorted({p for p, c, s in R}); seeds = sorted({s for p, c, s in R})
crash = sum(1 for r in rows if "error" in r["result"] or st(r) in ("", "crash", "error", "fetch_error"))
print(f"{SET}: 行数 {len(rows)}、問題 {len(probs)}、種 {seeds}、設定 {CF}、crash {crash}")
full = [(p, s) for p in probs for s in seeds if all((p, c, s) in R for c in CF)]
print("解けた数 (全設定がそろった", len(full), "組):", {c: sum(solved(R[(p, c, s)]) for p, s in full) for c in CF})
bad = []
for p, s in full:
    objs = [R[(p, c, s)]["result"].get("objective") for c in CF if st(R[(p, c, s)]) == "optimal"]
    if objs and max(objs) - min(objs) > 1e-6 * max(1, abs(objs[0])): bad.append((p, s, objs))
    if {st(R[(p, c, s)]) for c in CF} >= {"optimal", "infeasible"}: bad.append((p, s, "opt/inf"))
print("食い違い", len(bad), bad)
for c in CF:
    print(c, "最大 RSS", max([(r.get("max_rss_mb") or 0) for r in rows if r["cfg"] == c] or [0]), "MB")
def diag(r):
    """(外せる非零の割合 f (門の前の候補。CLQ は余裕のあるクリーク型の行全部), 外した行 (主), 戻した行 (主), 外した非零 (主))"""
    f = None; drop = back = dnnz = 0
    for l in r.get("lines", []):
        if l.startswith("GENLAZY {"):
            d = json.loads(l[8:])
            f = sum(v["dropnnz"] for v in d.values() if isinstance(v, dict)) / max(1, d["nnz"])
        elif l.startswith("CLQLAZY_F {"):
            f = json.loads(l[10:]).get("f_all")
        elif "LAZY stats tid 0" in l:
            w = l.split()
            drop = int(w[w.index("dropped") + 1]); back = int(w[w.index("restored") + 1])
            if "(nnz" in w: dnnz = int(w[w.index("(nnz") + 1].rstrip(")"))
    if f is None: return None
    return f, drop, back, dnnz
def cmp(ca, cb):
    logr = collections.defaultdict(list)
    for p, s in full:
        a, b = R[(p, ca, s)], R[(p, cb, s)]
        if solved(a) or solved(b): logr[p].append(math.log((t(b) + 1) / (t(a) + 1)))
    ps = list(logr)
    def sgm(q):
        v = [x for p in q for x in logr[p]]; return math.exp(sum(v) / len(v)) if v else 1.0
    random.seed(0); bs = sorted(sgm([random.choice(ps) for _ in ps]) for _ in range(4000)) if ps else [1.0] * 4000
    print(f"\n== {cb} / {ca} (どちらかが解けた {len(ps)} 問): {sgm(ps)-1:+.1%} [{bs[100]-1:+.1%}, {bs[3899]-1:+.1%}]")
    for p in sorted(ps, key=lambda q: -sum(logr[q]) / len(logr[q])):
        v = logr[p]; g = math.exp(sum(v) / len(v))
        tag = " 全種で遅" if len(v) >= 2 and all(x > math.log(1.25) for x in v) else (" 全種で速" if len(v) >= 2 and all(x < math.log(0.8) for x in v) else "")
        if g > 1.15 or g < 0.87:
            dg = [diag(R[(p, cb, s)]) for s in seeds if (p, cb, s) in R]
            dg = [x for x in dg if x]
            info = f" (f {dg[0][0]:.2f}、外した行 {[x[1] for x in dg]}、戻し {[x[2] for x in dg]})" if dg else ""
            print(f"  {p}: {g:.2f} {[round(math.exp(x), 2) for x in v]}{tag}{info}")
    return logr
L = {c: cmp("def", c) for c in CF if c != "def"}
# 問題ごとの特徴 (外せる非零の割合 f) と外した行・速さの比: 門 (シグモイド) が問題を分けたかの材料
print("\n== 問題ごと: f (最初の種の診断)・外した行 (主) と def に対する比 (どちらかが解けた種の幾何平均)")
print("問題 | " + " | ".join(f"{c} f / 外した行 / 比" for c in CF if c != "def"))
for p in probs:
    cells = []
    for c in CF:
        if c == "def": continue
        dg = next((diag(R[(p, c, s)]) for s in seeds if (p, c, s) in R and diag(R[(p, c, s)])), None)
        v = L[c].get(p)
        cells.append(f"{dg[0]:.2f} / {dg[1]} / " + (f"{math.exp(sum(v) / len(v)):.2f}" if v else "-") if dg else "- / - / " + (f"{math.exp(sum(v) / len(v)):.2f}" if v else "-"))
    print(p, "|", " | ".join(cells))
# M は (問題, 種) ごとに全設定の秒・終わりの下界を並べる (時間切れの問題 (puriri など) の下界の比べ)
if SET.startswith("M"):
    print("\n== (問題, 種) ごとの秒 (時間切れは「TL 下界」)")
    for p in probs:
        for s in seeds:
            cells = []
            for c in CF:
                r = R.get((p, c, s))
                if r is None: cells.append(f"{c} -"); continue
                cells.append(f"{c} {t(r):.0f}" if solved(r) else f"{c} TL {r['result'].get('bound')}")
            print(f"  {p} 種 {s}: " + " / ".join(cells))

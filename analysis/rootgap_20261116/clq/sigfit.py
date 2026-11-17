"""sigfit.jsonl (sigfit_run.py) から、問題ごとの点数 s と「root で回したときの得」の表を作り、ロジスティック回帰で a・b を合わせる。
python sigfit.py > sigfit.txt
- db: 根の下界の差 (root − fb) / max(1, |fb|) (最小化。正なら root が強い)。根で解き終わった問題は両方とも最適値なので 0
- tr: 根までの時間の比 root / fb
- y (root の得あり): db > 1e-4 かつ tr <= 3、または両方とも根で解けて tr < 0.8。y = 0 はそれ以外
  (下界も時間も差が小さい問題 (|db| <= 1e-4 かつ 0.8 <= tr <= 1.25) は「どちらでもよい」として回帰から外す)
回帰は z = a (s − b) の 1 変数 (重みは y の数で釣り合わせる)。4 つの量 (sp・ed・fr・vi) ごとの回帰も並べる。"""
import json, math
from pathlib import Path
D = Path(__file__).resolve().parent
R = {}
for l in open(D / "sigfit.jsonl"):
    if l.strip():
        r = json.loads(l); R[(r["problem"], r["cfg"])] = r
probs = sorted({p for p, c in R if (p, "fb") in R and (p, "root") in R})
def fbstate(p):
    t = (D / "logs_sig" / f"fb_{p}.log").read_text() if (D / "logs_sig" / f"fb_{p}.log").exists() else ""
    return "used" if "; used" in t else ("not used" if "not used" in t else "-")
rows = []
for p in probs:
    a, b = R[(p, "fb")], R[(p, "root")]
    if "s" not in a or a.get("root_bound") is None or b.get("root_bound") is None: continue
    rb_f, rb_r = a["root_bound"], b["root_bound"]
    db = (rb_r - rb_f) / max(1.0, abs(rb_f))
    tr = (b["time"] + 0.1) / (a["time"] + 0.1)
    both = a["status"] == "optimal" and b["status"] == "optimal"
    if db > 1e-4 and tr <= 3 or both and tr < 0.8: y = 1
    elif abs(db) <= 1e-4 and 0.8 <= tr <= 1.25: y = None
    else: y = 0
    rows.append(dict(p=p, s=a["s"], sp=a["sp"], ed=a["ed"], fr=a["fr"], vi=a["vi"], db=db, tf=a["time"], tt=b["time"], tr=tr, y=y, fb=fbstate(p), st=a["status"][:4] + "/" + b["status"][:4]))
rows.sort(key=lambda r: r["s"])
print(f"{len(rows)} 問 (s の昇順)。fb = 既定の fallback の判定 (used = clique を使った)")
print(f"{'問題':28s} {'s':>6s} {'sp':>6s} {'ed':>6s} {'fr':>6s} {'vi':>5s} {'db':>10s} {'t_fb':>7s} {'t_root':>7s} {'比':>5s}  y  fb の判定 状態")
for r in rows:
    print(f"{r['p']:28s} {r['s']:6.3f} {r['sp']:6.3f} {r['ed']:6.3f} {r['fr']:6.3f} {r['vi']:5.2f} {r['db']:+10.2e} {r['tf']:7.1f} {r['tt']:7.1f} {r['tr']:5.2f}  {'-' if r['y'] is None else r['y']}  {r['fb']:8s} {r['st']}")
def fit(key):
    d = [(r[key], r["y"]) for r in rows if r["y"] is not None]
    n1 = sum(y for _, y in d); n0 = len(d) - n1
    if n1 == 0 or n0 == 0: return None
    w1, w0 = len(d) / (2 * n1), len(d) / (2 * n0)
    # z = c0 + c1 x、ニュートン法 (小さな L2 正則化 1e-3)
    c0, c1 = 0.0, 0.0
    for _ in range(100):
        g0 = g1 = h00 = h01 = h11 = 0.0
        for x, y in d:
            w = w1 if y else w0
            pr = 1 / (1 + math.exp(-(c0 + c1 * x))); e = w * (pr - y); v = w * pr * (1 - pr)
            g0 += e; g1 += e * x; h00 += v; h01 += v * x; h11 += v * x * x
        g1 += 1e-3 * c1; h11 += 1e-3
        det = h00 * h11 - h01 * h01
        if abs(det) < 1e-15: break
        c0 -= (h11 * g0 - h01 * g1) / det; c1 -= (-h01 * g0 + h00 * g1) / det
    acc = sum((c0 + c1 * x > 0) == bool(y) for x, y in d)
    return c1, (-c0 / c1 if c1 else float("nan")), acc, len(d), n1
print()
for k in ("s", "sp", "ed", "fr", "vi"):
    f = fit(k)
    if f: print(f"回帰 {k}: a = {f[0]:.2f}, b = {f[1]:.3f}  (当たり {f[2]}/{f[3]}、y=1 は {f[4]} 問)")

# ---- 2 つ目: P・M の分担 (60 秒・600 秒、4 スレッド) の時間の比 root / c18 (問題ごとの幾何平均、どちらかが解けた種だけ) と s ----
import glob, collections
def st(r): return str(r["result"].get("status") or "").lower()
def tt(r): return min(r["result"]["time"], r["tl"]) if st(r) in ("optimal", "infeasible") else r["tl"]
RR = {}
for f in glob.glob(str(D / "res_[PM]_s*.jsonl")):
    for l in open(f):
        if l.strip():
            r = json.loads(l); RR[(r["problem"], r["cfg"], r["seed"])] = r
lr = collections.defaultdict(list)
for (p, c, sd), a in RR.items():
    if c != "c18" or (p, "c18_clqroot", sd) not in RR: continue
    b = RR[(p, "c18_clqroot", sd)]
    if st(a) in ("optimal", "infeasible") or st(b) in ("optimal", "infeasible"):
        lr[p].append(math.log((tt(b) + 1) / (tt(a) + 1)))
S = {r["p"]: r for r in rows}
rows2 = []
for p, v in lr.items():
    if p not in S: continue
    g = math.exp(sum(v) / len(v))
    y = 1 if g < 0.87 else (0 if g > 1.15 else None)
    rows2.append(dict(p=p, s=S[p]["s"], sp=S[p]["sp"], ed=S[p]["ed"], fr=S[p]["fr"], vi=S[p]["vi"], g=g, n=len(v), y=y))
rows2.sort(key=lambda r: r["s"])
print(f"\n== 全体の時間の比 root / c18 (P 60 秒 × 種 4・M 600 秒 × 種 2 の結果、{len(rows2)} 問) と s。y = 1: 比 < 0.87、0: 比 > 1.15、- はその間")
print(f"{'問題':28s} {'s':>6s} {'比':>6s} 種  y")
for r in rows2:
    print(f"{r['p']:28s} {r['s']:6.3f} {r['g']:6.2f} {r['n']:2d}  {'-' if r['y'] is None else r['y']}")
rows = rows2
print()
for k in ("s", "sp", "ed", "fr", "vi"):
    f = fit(k)
    if f: print(f"時間の比での回帰 {k}: a = {f[0]:.2f}, b = {f[1]:.3f}  (当たり {f[2]}/{f[3]}、y=1 は {f[4]} 問)")

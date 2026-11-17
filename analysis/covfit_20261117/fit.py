"""被覆カット (持ち上げつき被覆、ENOMOTO_MIP_ROW_LIFTED_COVER) の得失と、問題の量の相関 (新しい計測なし)。python fit.py > fit.txt
データは analysis/cutmix_20261111 (p58.jsonl: 58 問 × 種 1・2 × stat・lcstat、logs/p58_<問題>_<設定>_s<種>.txt の CUTMIX の型の割合と
SEPLEARN の分離器ごとの根の伸び)。得失は
- tr: 時間の比 log((t_lc + 10) / (t_stat + 10)) の種の平均 (負なら被覆で速い。時間切れは 60 秒)
- db: 最初の根の下界の差 (lcstat − stat) / max(1, |stat|) の種の平均 (最小化で正なら被覆で強い)
- rg: 最初の根の rows (元の行の分離。被覆はここに入る) の伸びの割合の差 (lcstat − stat、% 点)
量は CUTMIX の型の割合 (knap・knaptight・mixknap・introw・intsmall・genint・setp・cont) と、rows の伸びの割合 (stat)。Spearman の ρ を出す。"""
import json, math, re
from collections import defaultdict
from pathlib import Path
C = Path(__file__).resolve().parents[1] / "cutmix_20261111"
TL = 60.0
res = defaultdict(dict)
for l in (C / "p58.jsonl").read_text().splitlines():
    if not l.strip(): continue
    r = json.loads(l)
    if r["cfg"] not in ("stat", "lcstat"): continue
    res[(r["problem"], r["cfg"])][r["rep"]] = r["result"]
tp = re.compile(r"CUTMIX types restarts (\d+) m (\d+) n (\d+) (.*)")
sl = re.compile(r"SEPLEARN root restarts 0 root (\S+)\s+calls \d+ cands \d+ chosen (\d+) gain (\S+) \(([\d.]+)%\)")
def first_types(f):
    for l in f.read_text(errors="replace").splitlines():
        m = tp.match(l)
        if m and m.group(1) == "0":
            kv = m.group(4).split(); return {kv[i]: float(kv[i + 1]) for i in range(0, len(kv) - 1, 2)}
    return {}
def root_share(f):
    d = {}
    for l in f.read_text(errors="replace").splitlines():
        m = sl.match(l)
        if m and m.group(1) not in d: d[m.group(1)] = float(m.group(4))
    return d
rows = []
for p in sorted({p for p, c in res}):
    a, b = res.get((p, "stat"), {}), res.get((p, "lcstat"), {})
    reps = sorted(set(a) & set(b))
    if not reps: continue
    t = lambda r: min(r.get("time", TL), TL) if r.get("status") == "optimal" else TL
    if all(t(a[s]) < 1 and t(b[s]) < 1 for s in reps): continue
    tr = sum(math.log((t(b[s]) + 10) / (t(a[s]) + 10)) for s in reps) / len(reps)
    dbs = [((b[s].get("root_bound") or 0) - (a[s].get("root_bound") or 0)) / max(1, abs(a[s].get("root_bound") or 0)) for s in reps if a[s].get("root_bound") is not None and b[s].get("root_bound") is not None]
    fa, fb = C / "logs" / f"p58_{p}_stat_s1.txt", C / "logs" / f"p58_{p}_lcstat_s1.txt"
    if not fa.exists() or not fb.exists(): continue
    ty = first_types(fa); sa, sb = root_share(fa), root_share(fb)
    rows.append(dict(p=p, tr=tr, db=sum(dbs) / len(dbs) if dbs else 0.0, rg=sb.get("rows", 0) - sa.get("rows", 0), rows_share=sa.get("rows", 0), **{k: ty.get(k, 0.0) for k in ("knap", "knaptight", "mixknap", "introw", "intsmall", "genint", "setp", "cont")}))
def rank(v):
    o = sorted(range(len(v)), key=lambda i: v[i]); r = [0.0] * len(v); i = 0
    while i < len(o):
        j = i
        while j + 1 < len(o) and v[o[j + 1]] == v[o[i]]: j += 1
        for k in range(i, j + 1): r[o[k]] = (i + j) / 2
        i = j + 1
    return r
def spear(x, y):
    rx, ry = rank(x), rank(y); n = len(x); mx, my = sum(rx) / n, sum(ry) / n
    num = sum((a - mx) * (b - my) for a, b in zip(rx, ry)); den = math.sqrt(sum((a - mx) ** 2 for a in rx) * sum((b - my) ** 2 for b in ry))
    return num / den if den else 0.0
print(f"{len(rows)} 問 (両方 1 秒未満を除く)。tr < 0 は被覆で速い、db > 0 は被覆で根の下界が強い")
print(f"{'問題':22s} {'tr':>6s} {'db':>9s} {'rg':>6s} {'rows%':>6s} {'knap':>5s} {'kntg':>5s} {'mixk':>5s} {'intr':>5s}")
for r in sorted(rows, key=lambda r: r["tr"]):
    print(f"{r['p']:22s} {r['tr']:+6.2f} {r['db']:+9.2e} {r['rg']:+6.1f} {r['rows_share']:6.1f} {r['knap']:5.2f} {r['knaptight']:5.2f} {r['mixknap']:5.2f} {r['introw']:5.2f}")
print("\nSpearman の ρ (量 × 得失):")
for k in ("knap", "knaptight", "mixknap", "introw", "intsmall", "genint", "setp", "cont", "rows_share"):
    x = [r[k] for r in rows]
    print(f"  {k:11s} tr {spear(x, [r['tr'] for r in rows]):+.2f}  db {spear(x, [r['db'] for r in rows]):+.2f}  rg {spear(x, [r['rg'] for r in rows]):+.2f}")
print(f"\n得失どうし: tr × db {spear([r['tr'] for r in rows], [r['db'] for r in rows]):+.2f}、tr × rg {spear([r['tr'] for r in rows], [r['rg'] for r in rows]):+.2f}")

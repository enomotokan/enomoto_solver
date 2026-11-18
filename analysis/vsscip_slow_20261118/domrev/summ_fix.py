"""drev の採否の表: 固定のあった問題と固定 0 列 (既定と同じ計算) の問題に分けて、drev / def の時間の比 (shift 1 秒、時間切れは上限) を出す。
python summ_fix.py [P|M] [drev] > summ_fix_<SET>.txt。固定の数は fixcount.txt (-1 = domcol を呼ぶ前に前処理が終わった問題、0 と同じ扱い)。"""
import json, glob, math, sys, collections
from pathlib import Path
D = Path(__file__).resolve().parent
SET = sys.argv[1] if len(sys.argv) > 1 else "P"; ALT = sys.argv[2] if len(sys.argv) > 2 else "drev"
FIX = {}
for l in open(D / "fixcount.txt"):
    p, nf, nx, fr = l.split(); FIX[p] = (int(nf), int(nx))
R = collections.defaultdict(dict)
for f in glob.glob(str(D / f"res_{SET}_s*.jsonl")):
    for l in open(f):
        if not l.strip(): continue
        r = json.loads(l)
        if r["cfg"] not in ("def", ALT): continue
        x = r["result"]; tl = r.get("tl", 600 if SET == "M" else 60)
        t = x.get("time") if x.get("status") == "optimal" else tl
        R[(r["problem"], r["seed"])][r["cfg"]] = (min(t or tl, tl), x.get("status"))
rows = collections.defaultdict(list)
for (p, s), v in R.items():
    if "def" in v and ALT in v: rows[p].append((s, v["def"], v[ALT]))
def sgm(pairs):
    return math.exp(sum(math.log((b + 1) / (a + 1)) for a, b in pairs) / len(pairs))
for grp, sel in (("固定あり", lambda p: FIX.get(p, (0, 0))[1] > 0), ("固定 0 (既定と同じ計算)", lambda p: FIX.get(p, (0, 0))[1] <= 0)):
    ps = sorted(p for p in rows if sel(p))
    allp = [(d[0], a[0]) for p in ps for _, d, a in rows[p]]
    sd = sum(1 for p in ps for _, d, a in rows[p] if d[1] == "optimal"); sa = sum(1 for p in ps for _, d, a in rows[p] if a[1] == "optimal")
    print(f"== {SET} {grp}: {len(ps)} 問・{len(allp)} 組、解けた def {sd} / {ALT} {sa}、{ALT}/def の SGM {sgm(allp):.3f}" if allp else f"== {grp}: なし")
    for p in ps:
        pr = [(d[0], a[0]) for _, d, a in rows[p]]
        nf, nx = FIX.get(p, (0, 0))
        fr = f"{nx}/{nf} ({100 * nx / nf:.1f}%)" if nx > 0 and nf > 0 else "0"
        if grp == "固定あり" or abs(math.log(sgm(pr))) > math.log(1.25):
            print(f"  {p}: 固定 {fr}、比 {sgm(pr):.2f} [" + ", ".join(f"s{s} {d[0]:.1f}{'' if d[1]=='optimal' else 'TL'}→{a[0]:.1f}{'' if a[1]=='optimal' else 'TL'}" for s, d, a in sorted(rows[p])) + "]")

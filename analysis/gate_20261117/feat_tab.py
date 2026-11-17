"""GATE: 既存の対の比べ (slow2 の res_*) から、問題ごとの比 (alt/def) と結果に出ている特徴 (def の時間・節点・根の下界の閉じ具合・最初の解の時刻) を並べる。
python feat_tab.py MSW sw"""
import json, glob, math, collections, sys
from pathlib import Path
S = Path(__file__).resolve().parents[1] / "slow2_20261117"
SET, ALT = sys.argv[1], sys.argv[2]
R = {}
for f in glob.glob(str(S / f"res_{SET}_s*.jsonl")):
    for l in open(f):
        if l.strip():
            r = json.loads(l); R[(r["problem"], r["cfg"], r["seed"])] = r
def solved(r): return str(r["result"].get("status")).lower() in ("optimal", "infeasible")
def t(r): return min(r["result"]["time"], r["tl"]) if solved(r) else r["tl"]
by = collections.defaultdict(list)
for (p, c, s), r in R.items():
    if c != "def" or (p, ALT, s) not in R: continue
    a, b = r, R[(p, ALT, s)]
    if not (solved(a) or solved(b)): continue
    ra = a["result"]; opt = ra.get("objective"); rb = ra.get("root_bound")
    gap = abs(opt - rb) / max(1, abs(opt)) if opt is not None and rb is not None else float("nan")
    by[p].append(((t(b) + 1) / (t(a) + 1), t(a), ra.get("nodes"), b["result"].get("nodes"), gap, ra.get("first_solution_time"), s))
for p in sorted(by, key=lambda q: -sum(math.log(x[0]) for x in by[q]) / len(by[q])):
    v = by[p]; g = math.exp(sum(math.log(x[0]) for x in v) / len(v))
    print(f"{p:28s} {g:5.2f} " + " | ".join(f"s{x[6]} r{x[0]:.2f} t{x[1]:.0f} n{x[2]}->{x[3]} gap{x[4]:.1e} fs{(x[5] or -1):.1f}" for x in v))

"""bench58.jsonl を集計する: 設定ごとの解けた数・SGM (10 秒ずらし、解けない・60 秒超えは 60 秒)、試作が働いた問題 (列挙の前処理が
固定・同値を出した、または対称性の生成元の数が既定と違う)、1.3 倍以上違った問題。使い方: python agg58.py [基準 (def)] [比べる設定 (ho3)]"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
R = defaultdict(dict)
base = sys.argv[1] if len(sys.argv) > 1 else "def"
cmp_ = sys.argv[2] if len(sys.argv) > 2 else "ho3"
for l in open(D / "bench58.jsonl"):
    r = json.loads(l); R[r["cfg"]][r["problem"]] = r["result"]
def t(res):
    ok = res.get("status") in ("optimal", "infeasible") and res.get("time", 99) <= 60.0
    return res["time"] if ok else 60.0
ps = [p for p in R[base] if p in R[cmp_]]
def sgm(c, qs): return math.exp(sum(math.log(t(R[c][p]) + 10) for p in qs) / len(qs)) - 10 if qs else float("nan")
for c in (base, cmp_):
    print(f"{c}: {len(ps)} 問、解けた {sum(1 for p in ps if t(R[c][p]) < 60)}、SGM(+10) {sgm(c, ps):.2f} 秒")
fired = [p for p in ps if any(R[cmp_][p].get("enum") or []) or R[cmp_][p].get("gens") != R[base][p].get("gens")]
print(f"\n試作が働いた問題 {len(fired)} 問: SGM {sgm(base, fired):.2f} → {sgm(cmp_, fired):.2f}")
print("\n| 問題 | 基準 | 試作 | 比 | 列挙 (固定/締め/同値) | 生成元 (基準 → 試作) | 根の LP (基準 → 試作) |")
print("|---|---|---|---|---|---|---|")
for p in ps:
    a, b = R[base][p], R[cmp_][p]
    ta, tb = t(a), t(b)
    big = (tb + 10) / (ta + 10)
    if p in fired or big > 1.3 or big < 1 / 1.3:
        f = lambda r: (f"{r['time']:.1f}" if t(r) < 60 else f"t ({r.get('objective')}/{r.get('bound'):.6g})" if isinstance(r.get('bound'), float) else str(r.get("status")))
        print(f"| {p} | {f(a)} | {f(b)} | {big:.2f} | {'/'.join(map(str, b.get('enum') or []))} | {a.get('gens')} → {b.get('gens')} | {a.get('root_lp')} → {b.get('root_lp')} |")

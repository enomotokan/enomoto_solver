"""bench.jsonl の集計: 回ごと・設定ごとの SGM (時間 + 1 の幾何平均 - 1、時間切れは 60 秒) と解けた数、片方だけ解けた問題、時間の差の大きい問題。"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
R = defaultdict(dict)
for l in (D / "bench.jsonl").read_text().splitlines():
    r = json.loads(l); res = r["result"]
    ok = res.get("status") == "optimal" and res.get("time", 99) <= 60.0
    R[(r["rep"], r["cfg"])][r["problem"]] = (res.get("time", 60.0) if ok else 60.0, ok, res.get("objective"))
cfgs = sorted({c for _, c in R}); reps = sorted({r for r, _ in R})
for rep in reps + ["all"]:
    keys = [(r, c) for r in (reps if rep == "all" else [rep]) for c in cfgs]
    common = None
    for r in (reps if rep == "all" else [rep]):
        s = set.intersection(*[set(R[(r, c)]) for c in cfgs if (r, c) in R])
        common = (common or set()) | {(r, p) for p in s}
    line = f"回 {rep}: {len(common)} 件"
    for c in cfgs:
        ts = [R[(r, c)][p] for (r, p) in common]
        sgm = math.exp(sum(math.log(t + 1) for t, _, _ in ts) / len(ts)) - 1
        line += f" | {c} {sgm:.2f} ({sum(o for _, o, _ in ts)})"
    print(line)
base = "hs_pi0_bgw"
for b in [c for c in cfgs if c != base]:
    a = base
    print(f"\n片方だけ解けた・時間の比が 1.5 倍を超える (差 3 秒以上) 問題 ({a} / {b}):")
    for rep in reps:
        for p in sorted(set(R[(rep, a)]) & set(R[(rep, b)])):
            ta, oa, za = R[(rep, a)][p]; tb, ob, zb = R[(rep, b)][p]
            if oa != ob or (abs(ta - tb) >= 3 and max(ta, tb) / max(min(ta, tb), 0.1) > 1.5):
                print(f"  回 {rep} {p:22s} {ta:5.1f}{'' if oa else '*'} / {tb:5.1f}{'' if ob else '*'}")
            if oa and ob and za is not None and zb is not None and abs(za - zb) > 1e-6 * max(1, abs(za)):
                print(f"  !! 最適値の食い違い 回 {rep} {p}: {za} / {zb}")

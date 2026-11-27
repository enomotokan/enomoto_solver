"""分担表: python plan.py [分担の数 (既定 6)] → shards.json (picked.json の 48 問 × {ours_t1, ours_t4, scip, highs_t1, highs_t4} × 種 1、600 秒)。
同じ問題の 5 設定は同じ分担で続けて解き、順は問題ごとに回して入れ替える。見積り (1 回、秒) は次の記録の時間 (解けなければ 600) + 待ち 30 秒:
- ours_t1・ours_t4: vsscip_20261120 の M の def_t1・def (今の既定に近い)。P の問題は vsscip_20261117 の P (60 秒) で解けた回の時間、解けなければ 600。
- scip: vsscip_20261117 の M・P の scip。highs_t4: vshighs_20261117 の M・A・P。highs_t1: highs_t4 の 2 倍 (計測がない。上限 600)。
- 記録がなければ 600 (comp07-2idx の ours は 600 とみる)。"""
import collections, glob, json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
A = D.parent
NS = int(sys.argv[1]) if len(sys.argv) > 1 else 6
TL = 600
CF = ["ours_t1", "ours_t4", "scip", "highs_t1", "highs_t4"]
probs = sorted(json.loads((D / "picked.json").read_text(encoding="utf-8")))
def st(r): return str(r["result"].get("status") or "").lower()
def solved(r): return st(r) in ("optimal", "infeasible")
T = collections.defaultdict(list)  # (問題, 設定) → 時間 (600 秒に直したもの)
for pat, mp in [("vsscip_20261120/res_M_s*.jsonl", {"def": "ours_t4", "def_t1": "ours_t1"}), ("vsscip_20261117/res_M_s*.jsonl", {"scip": "scip"}),
                ("vsscip_20261117/res_P_s*.jsonl", {"def": "ours_t4", "def_t1": "ours_t1", "scip": "scip"}),
                ("vshighs_20261117/res_M_s*.jsonl", {"highs": "highs_t4"}), ("vshighs_20261117/res_A_s*.jsonl", {"highs": "highs_t4"}),
                ("vshighs_20261117/res_P_s*.jsonl", {"highs": "highs_t4"})]:
    for f in glob.glob(str(A / pat)):
        for l in open(f):
            if not l.strip(): continue
            r = json.loads(l)
            c = mp.get(r["cfg"])
            if c and r["problem"] in probs:
                T[(r["problem"], c)].append(min(r["result"]["time"], TL) if solved(r) else TL)
def est(p, c):
    if c == "highs_t1":
        return min(TL, 2 * est(p, "highs_t4") - 30) + 30
    v = T.get((p, c))
    return (sum(v) / len(v) if v else TL) + 30
groups = {p: [(p, c, 1) for c in CF[i % 5:] + CF[:i % 5]] for i, p in enumerate(probs)}
gl = sorted(groups.items(), key=lambda kv: -sum(est(p, c) for p, c, s in kv[1]))
load = [0.0] * NS; shards = [[] for _ in range(NS)]
for _, ts in gl:
    i = min(range(NS), key=lambda j: load[j]); shards[i] += [[p, c, s, TL] for p, c, s in ts]; load[i] += sum(est(p, c) for p, c, s in ts)
(D / "shards.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
tot = {c: sum(est(p, c) for p in probs) / 3600 for c in CF}
print(f"shards.json: {len(probs)} 問 × {len(CF)} 設定 = {sum(len(s) for s in shards)} 回、{NS} 分担、見積り (時間) 各 {min(load)/3600:.2f}〜{max(load)/3600:.2f}、計 {sum(load)/3600:.1f}")
print("設定ごと (時間):", {c: round(v, 1) for c, v in tot.items()})

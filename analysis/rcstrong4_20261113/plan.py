"""RCSTRONG4 の分担表を作る: python plan.py NT NP → shards_T.json・shards_P.json。同じ (問題, 種) の形は同じ分担に入れる。
(T) 狙いの問題 × 種 1〜4 × {k, ik, gk} 600 秒: rmatr100-p10 (i の害、rcstrong2)・neos-2978193-inde・fiball (i の効き、cand1114/fiball.md)・
    pk1 (ノードの切り捨ての害、rcstrong)・neos-911970・swath1 (irk の害、rcstrong2)・physiciansched6-2 (k が速い、rcstrong2 の 58 問)・
    mc11・10teams・neos5 (k が遅い、同)。
(P) 58 問 × 種 1・2 × {k, gk} 60 秒。時間の見積りは cand1113 の P (60 秒) と M (600 秒) の回。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
NT, NP = int(sys.argv[1]), int(sys.argv[2])
SUF = sys.argv[3] if len(sys.argv) > 3 else ""  # "2": 直した gk (GK_INT) の 2 回目。T も {k, gk} だけ
FORMS_T = ["k", "ik", "gk"] if not (len(sys.argv) > 3) else ["k", "gk"]
FORMS_P = ["k", "gk"]
T_PROBS = ["rmatr100-p10", "neos-2978193-inde", "fiball", "pk1", "neos-911970", "swath1", "physiciansched6-2", "mc11", "10teams", "neos5"]
def times(pat, cap):
    T = defaultdict(list)
    for f in glob.glob(str(WT / "analysis/cand1113_20261112" / pat)):
        for l in open(f):
            r = json.loads(l); t = r["result"].get("time") or cap
            T[r["problem"]].append(min(t, cap))
    return T
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -est(kv[0][0]) * len(kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for (p, s), cs in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [[p, c, s, tl] for c in cs]; load[i] += est(p) * len(cs)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, sum(map(len, shards)), "回、見積り (時間):", [round(x / 3600, 2) for x in load])
TP, TM = times("res_P_s*.jsonl", 60), times("res_M_s*.jsonl", 600)
def est_t(p):
    v = TM.get(p) or [min(4 * x, 600) for x in TP.get(p, [])] or [300]
    return min(sum(v) / len(v) * 1.5, 600) + 20
balance({(p, s): FORMS_T for p in T_PROBS for s in (1, 2, 3, 4)}, est_t, NT, f"shards_T{SUF}.json", 600)
def est_p(p):
    v = TP.get(p) or [40]
    return sum(v) / len(v) + 8
balance({(p, s): FORMS_P for p in PROBS for s in (1, 2)}, est_p, NP, f"shards_P{SUF}.json", 60)

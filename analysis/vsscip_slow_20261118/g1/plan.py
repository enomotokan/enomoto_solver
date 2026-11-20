"""群 1 の対の比べの分担表: python plan.py NM NP NT [g1] → shards_M.json・shards_P.json・shards_T.json
M: Mittelmann 48 問 (slow2・vshighs の M と同じ) × 種 1・2 × {def, g1}、600 秒。P: MIPLIB の 58 問 × 種 1〜3 × {def, g1}、60 秒。
T: 群 1 の 5 問 (graph20-20-1rand・neos-787933・bnatt400・bnatt500・neos-3381206-awhea) × 種 1・2 × {def, g1}、600 秒。
同じ (問題, 種) の 2 組は同じ分担で続けて解き、順は種で入れ替える。見積り: vshighs_20261117 (M・P) と vsscip_20261117 (T) の def の時間 (+ 30 秒・8 秒)。
解けなかった回は上限で数える。g1 も def と同じと置く。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
A = D.parents[1]
sys.path.insert(0, str(A / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
NM, NP, NT = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
ALT = sys.argv[4] if len(sys.argv) > 4 else "g1"
def solved(r): return str(r["result"].get("status", "")).lower() in ("optimal", "infeasible")
def times(pat, cap):
    T = defaultdict(list)
    for f in glob.glob(str(pat)):
        for l in open(f):
            r = json.loads(l)
            if r["cfg"] != "def": continue
            T[r["problem"]].append(min(r["result"]["time"], cap) if solved(r) else cap)
    return {k: sum(v) / len(v) for k, v in T.items()}
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -sum(est(t[0], t[1]) for t in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for (p, s), ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += sum(est(t[0], t[1]) for t in ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜", max(round(x / 3600, 2) for x in load), "計", round(sum(load) / 3600, 1))
def order(p, s, c):
    k = s % 2
    return [(p, x, s) for x in c[k:] + c[:k]]
TM = times(A / "vshighs_20261117/res_M_s*.jsonl", 600)
balance({(p, s): order(p, s, ["def", ALT]) for p in sorted(TM) for s in (1, 2)}, lambda p, c: TM.get(p, 600.0) + 30, NM, "shards_M.json", 600)
TP = times(A / "vshighs_20261117/res_P_s*.jsonl", 60)
balance({(p, s): order(p, s, ["def", ALT]) for p in PROBS for s in (1, 2, 3)}, lambda p, c: TP.get(p, 60.0) + 8, NP, "shards_P.json", 60)
G1 = ["graph20-20-1rand", "neos-787933", "bnatt400", "bnatt500", "neos-3381206-awhea"]
TT = times(A / "vsscip_20261117/res_M_s*.jsonl", 600)
balance({(p, s): order(p, s, ["def", ALT]) for p in G1 for s in (1, 2)}, lambda p, c: TT.get(p, 600.0) + 30, NT, "shards_T.json", 600)

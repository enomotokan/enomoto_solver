"""0.1.9 の既定と SCIP の比べの分担表: python plan.py NM NP → shards_M.json・shards_P.json
M: Mittelmann 186 問 (analysis/cbcimpl_20261029/mitt_parts.json) × 種 1 × {def, def_t1, scip}、600 秒。
P: MIPLIB の 58 問 (analysis/cbcimpl_20261029/probs.py) × 種 1〜3 × {def, def_t1, scip}、60 秒。
同じ (問題, 種) の 3 組は同じ分担で続けて解き、順は (問題, 種) ごとに回して入れ替える (時期・機械のずれを組で消す)。
見積り: vshighs_20261117 の def の時間 (res_M_s*・res_A_s*・res_P_s*、種の平均)。def で解けなかった問題は上限。
def_t1 と scip は不明なので min(2 × def, 上限) と置く (def で解けなかった問題は上限)。1 回ごとに M は +30 秒、P は +8 秒。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
VH = D.parent / "vshighs_20261117"
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
NM, NP = int(sys.argv[1]), int(sys.argv[2])
CF = ["def", "def_t1", "scip"]
def solved(r): return str(r["result"].get("status", "")).lower() in ("optimal", "infeasible")
def times(pats, cap):
    T = defaultdict(list)
    for pat in pats:
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
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜",
          max(round(x / 3600, 2) for x in load), "計", round(sum(load) / 3600, 1))
def order(p, s, i):
    k = i % 3
    return [(p, x, s) for x in CF[k:] + CF[:k]]
def mk(T, tl, pad):
    def est(p, c):
        t = T.get(p, tl)
        return (t if c == "def" else min(2 * t, tl)) + pad
    return est
MPROBS = sorted({x for part in json.loads((D.parent / "cbcimpl_20261029/mitt_parts.json").read_text()) for x in part})
TM = times([VH / "res_M_s*.jsonl", VH / "res_A_s*.jsonl"], 600)
balance({(p, 1): order(p, 1, i) for i, p in enumerate(MPROBS)}, mk(TM, 600, 30), NM, "shards_M.json", 600)
TP = times([VH / "res_P_s*.jsonl"], 60)
balance({(p, s): order(p, s, i + s) for i, p in enumerate(PROBS) for s in (1, 2, 3)}, mk(TP, 60, 8), NP, "shards_P.json", 60)

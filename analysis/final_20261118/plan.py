"""最終計測の分担表: python plan.py NM NP → shards_M.json・shards_P.json
M: Mittelmann 186 問 (analysis/cbcimpl_20261029/mitt_parts.json) × 種 1 × {def, def_t1, v019}、600 秒。
P: MIPLIB の 58 問 (analysis/cbcimpl_20261029/probs.py) × 種 1〜3 × {def, def_t1, v019}、60 秒。
見積り: vsscip_20261117 の 0.1.9 の同じ設定の時間 (v019 は def と同じ) (解けなかった回は上限)。1 回ごとに M は +30 秒、P は +8 秒。"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent
VS = D.parent / "vsscip_20261117"
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
NM, NP = int(sys.argv[1]), int(sys.argv[2])
CF = ["def", "def_t1", "v019"]
def solved(r): return str(r["result"].get("status", "")).lower() in ("optimal", "infeasible")
def times(pat, cap):
    T = {}
    for f in glob.glob(str(VS / pat)):
        for l in open(f):
            r = json.loads(l)
            if r["cfg"] in ("def", "def_t1"): T[(r["problem"], r["cfg"], r["seed"])] = min(r["result"]["time"], cap) if solved(r) else cap
    return T
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -sum(est(*t) for t in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for _, ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += sum(est(*t) for t in ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜",
          max(round(x / 3600, 2) for x in load), "計", round(sum(load) / 3600, 1))
MPROBS = sorted({x for part in json.loads((D.parent / "cbcimpl_20261029/mitt_parts.json").read_text()) for x in part})
TM = times("res_M_s*.jsonl", 600)
balance({(p, 1): [(p, c, 1) for c in (CF[i % 3:] + CF[:i % 3])] for i, p in enumerate(MPROBS)}, lambda p, c, s: TM.get((p, "def" if c == "v019" else c, s), 600) + 30, NM, "shards_M.json", 600)
TP = times("res_P_s*.jsonl", 60)
balance({(p, s): [(p, c, s) for c in (CF[(i + s) % 3:] + CF[:(i + s) % 3])] for i, p in enumerate(PROBS) for s in (1, 2, 3)}, lambda p, c, s: TP.get((p, "def" if c == "v019" else c, s), 60) + 8, NP, "shards_P.json", 60)

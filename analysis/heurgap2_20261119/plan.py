"""P2-HEUR の分担表: python plan.py NM NP → shards_M.json・shards_P.json
P: MIPLIB の 58 問 (analysis/cbcimpl_20261029/probs.py) × 種 1〜3 × {def, rg}、60 秒、4 スレッド。
M: 対象の 7 問と、最終計測 (analysis/final_20261118) の def で時間切れ・間隙の大きかった Mittelmann 17 問 × 種 1・2 × {def_t1, rg_t1}、600 秒、1 スレッド (試作が効くのは主に 1 スレッド、cfg.py)。
見積り: 最終計測の def (M は def_t1) の時間 (解けなかった回は上限) に、1 回ごとに M は +30 秒、P は +8 秒。同じ (問題, 種) の 2 回は同じ分担に入れ、順は交互にする。"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent
FIN = D.parent / "final_20261118"
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
NM, NP = int(sys.argv[1]), int(sys.argv[2])
CF = ["def", "rg"]
CM = ["def_t1", "rg_t1"]
TARGET = ["trento1", "neos-4300652-rahue", "nexp-150-20-8-5", "sing326", "reblock115", "brazil3", "neos-4763324-toguru"]
NEAR = ["neos-3754480-nidda", "rocII-5-11", "traininstance2", "neos-5188808-nattai", "neos-2657525-crna", "academictimetablesmall",
        "neos-5195221-niemur", "neos-5107597-kakapo", "comp21-2idx", "opm2-z10-s4", "neos-3046615-murg", "lectsched-5-obj",
        "dws008-01", "glass4", "neos-3656078-kumeu", "rococoB10-011000", "rmatr200-p5"]
def solved(r): return str(r["result"].get("status", "")).lower() in ("optimal", "infeasible")
def times(pat, cap, cfg="def"):
    T = {}
    for f in glob.glob(str(FIN / pat)):
        for l in open(f):
            r = json.loads(l)
            if r["cfg"] == cfg: T[(r["problem"], r["seed"])] = min(r["result"]["time"], cap) if solved(r) else cap
    return T
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -sum(est(*t) for t in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for _, ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += sum(est(*t) for t in ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜",
          max(round(x / 3600, 2) for x in load), "計", round(sum(load) / 3600, 1))
TM = times("res_M_s*.jsonl", 600, "def_t1")
balance({(p, s): [(p, c, s) for c in (CM if (i + s) % 2 else CM[::-1])] for i, p in enumerate(TARGET + NEAR) for s in (1, 2)},
        lambda p, c, s: TM.get((p, 1), 600) + 30, NM, "shards_M.json", 600)
TP = times("res_P_s*.jsonl", 60)
balance({(p, s): [(p, c, s) for c in (CF if (i + s) % 2 else CF[::-1])] for i, p in enumerate(PROBS) for s in (1, 2, 3)},
        lambda p, c, s: TP.get((p, s), 60) + 8, NP, "shards_P.json", 60)

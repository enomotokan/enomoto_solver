"""再スタートの費用の試作 (README の 10 節) の広い計測の分担表: python plan_rs.py NM NP → shards_MRS.json・shards_PRS.json。
PRS: MIPLIB の 58 問 × 種 1〜3 × {def, c, cb, cba, pl, pl_cb}、60 秒。MRS: plan.py の M の 46 問 × 種 1・2 × {def, cb}、600 秒。4 スレッド。
同じ (問題, 種) の設定は同じ分担で続けて解き、順は種で回す。見積り: 最終計測 (analysis/final_20261118) の def の時間 (+ 30 秒・8 秒)。
回し方: python run.py PRS K・python run.py MRS K。集計: python summ.py PRS c など (def との比)。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.argv = [sys.argv[0], sys.argv[1], sys.argv[2], "l"]
import plan  # noqa: E402  (plan.py の問題の並び・見積りを使う。読み込むと shards_ML/PL を書き直すので後で戻す)
NM, NP = int(sys.argv[1]), int(sys.argv[2])
def balance(probs, seeds, cfgs, est, n, out, tl):
    groups = {(p, s): [(p, cfgs[(i + s) % len(cfgs)], s) for i in range(len(cfgs))] for p in probs for s in seeds}
    gl = sorted(groups.items(), key=lambda kv: -est(kv[0][0]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for (p, s), ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += len(cfgs) * est(p)
    (D / out).write_text(json.dumps({"problems": probs, "shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(probs), "問", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜", max(round(x / 3600, 2) for x in load))
balance(plan.MPROBS, (1, 2), ["def", "cb"], lambda p: plan.TM.get(p, 600.0) + 30, NM, "shards_MRS.json", 600)
balance(list(plan.PROBS), (1, 2, 3), ["def", "c", "cb", "cba", "pl", "pl_cb"], lambda p: plan.TP.get(p, 48.0) + 8, NP, "shards_PRS.json", 60)

"""P2-PAR の対の比べの分担表: python plan.py NM NP 設定 M の問題 (カンマ区切り) → shards_M<札>.json・shards_P<札>.json (札は設定の大文字)。
M: 渡した問題 × 種 1・2 × {def, 設定}、600 秒。P: MIPLIB の 58 問 × 種 1〜3 × {def, 設定}、60 秒。4 スレッド。
同じ (問題, 種) の 2 組は同じ分担で続けて解き、順は種で入れ替える。見積り: analysis/final_20261118 の def の時間 (+ 30 秒・8 秒)。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
FIN = D.parent / "final_20261118"
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
NM, NP, ALT = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
MPROBS = sys.argv[4].split(",")
SUF = ALT.upper()
def times(pat, cap):
    T = defaultdict(list)
    for f in glob.glob(str(pat)):
        for l in open(f):
            r = json.loads(l)
            if r["cfg"] != "def": continue
            ok = str(r["result"].get("status", "")).lower() in ("optimal", "infeasible")
            T[r["problem"]].append(min(r["result"].get("time") or cap, cap) if ok else cap)
    return {k: sum(v) / len(v) for k, v in T.items()}
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -sum(est(t[0]) for t in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for _, ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += sum(est(t[0]) for t in ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜", max(round(x / 3600, 2) for x in load))
def order(p, s, c):
    k = s % 2
    return [(p, x, s) for x in c[k:] + c[:k]]
TM = times(FIN / "res_M_s*.jsonl", 600)
balance({(p, s): order(p, s, ["def", ALT]) for p in MPROBS for s in (1, 2)}, lambda p: TM.get(p, 600.0) + 30, NM, f"shards_M{SUF}.json", 600)
TP = times(FIN / "res_P_s*.jsonl", 60)
balance({(p, s): order(p, s, ["def", ALT]) for p in PROBS for s in (1, 2, 3)}, lambda p: TP.get(p, 48.0) + 8, NP, f"shards_P{SUF}.json", 60)

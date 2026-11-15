"""cand1115c・d の確かめの分担表: python plan3.py NM NP → shards_M3.json・shards_P3.json
P3: 58 問 × 種 1〜4 × {cand1114, cand1115c, cand1115d} 60 秒。M3: plan.py の M と同じ組 × 種 1・2 × 同じ 3 組 600 秒。
同じ (問題, 種) の 3 組は同じ分担で続けて解き、順は種で回す。見積りは cand1114 の回。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
C14 = D.parent / "cand1114_20261113"
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
NM, NP = int(sys.argv[1]), int(sys.argv[2])
def times(pat, cap):
    T = defaultdict(list)
    for f in glob.glob(str(C14 / pat)):
        for l in open(f):
            r = json.loads(l)
            if r["cfg"] != "cand1114": continue
            t = r["result"].get("time") or cap
            T[r["problem"]].append(min(t, cap))
    return T
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -est[kv[0][0]] * len(kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for (p, s), ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += est[p] * len(ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜", max(round(x / 3600, 2) for x in load))
def pair(p, s):
    c = ["cand1114", "cand1115c", "cand1115d"]
    k = s % 3
    return [(p, x, s) for x in c[k:] + c[:k]]
TM = times("res_M_s*.jsonl", 600)
MPROBS = sorted(TM)
estM = defaultdict(lambda: 630.0, {k: sum(v) / len(v) + 30 for k, v in TM.items()})
balance({(p, s): pair(p, s) for p in MPROBS for s in (1, 2)}, estM, NM, "shards_M3.json", 600)
TP = times("res_P_s*.jsonl", 60)
estP = defaultdict(lambda: 48.0, {k: sum(v) / len(v) + 8 for k, v in TP.items()})
balance({(p, s): pair(p, s) for p in PROBS for s in (1, 2, 3, 4)}, estP, NP, "shards_P3.json", 60)

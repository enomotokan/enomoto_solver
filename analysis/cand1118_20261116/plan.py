"""cand1118 の確かめの分担表: python plan.py NM NP → shards_M.json・shards_P.json
P: 58 問 × 種 1〜4 × {cand1114, cand1117, cand1118} 60 秒。M: cand1117 の M と同じ 48 問 × 種 1・2 × {cand1117, cand1118, highs} 600 秒。
同じ (問題, 種) の 3 組は同じ分担で続けて解き、順は種で回す (時期・機械のずれを対で消す)。
見積り: P・M とも cand1117 の回の cand1117 の時間 (+ 立ち上げ)、highs は cand1117 の回の highs の時間。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
C17 = D.parent / "cand1117_20261116"
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
NM, NP = int(sys.argv[1]), int(sys.argv[2])
def times(pat, cfg, cap):
    T = defaultdict(list)
    for f in glob.glob(str(pat)):
        for l in open(f):
            r = json.loads(l)
            if r["cfg"] != cfg: continue
            t = r["result"].get("time") or cap
            T[r["problem"]].append(min(t, cap))
    return {k: sum(v) / len(v) for k, v in T.items()}
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -sum(est(t[0], t[1]) for t in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for (p, s), ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += sum(est(t[0], t[1]) for t in ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜", max(round(x / 3600, 2) for x in load))
def order(p, s, c):
    k = s % 3
    return [(p, x, s) for x in c[k:] + c[:k]]
TM = times(C17 / "res_M_s*.jsonl", "cand1117", 600)
TH = times(C17 / "res_M_s*.jsonl", "highs", 600)
MPROBS = sorted(TM)
def estM(p, c):
    t = TH.get(p, TM.get(p, 600.0)) if c == "highs" else TM.get(p, 600.0)
    return t + 30
balance({(p, s): order(p, s, ["cand1117", "cand1118", "highs"]) for p in MPROBS for s in (1, 2)}, estM, NM, "shards_M.json", 600)
TP = times(C17 / "res_P_s*.jsonl", "cand1117", 60)
def estP(p, c): return TP.get(p, 48.0) + 8
balance({(p, s): order(p, s, ["cand1114", "cand1117", "cand1118"]) for p in PROBS for s in (1, 2, 3, 4)}, estP, NP, "shards_P.json", 60)

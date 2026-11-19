"""EQUIV2_PWORK の確かめの分担表: python plan.py → shards_P.json・shards_M.json。
P: MIPLIB の 58 問 × 種 1〜3 × {def, epw}、60 秒。M: 11-07 の計測 (analysis/prepar_20261107/pretime_mitt_t600.jsonl) でプロービングが 2 秒以上、
かつ final_20261118 の M の既定で 600 秒以内に解けた 16 問 × 種 1〜3 × {def, epw}、600 秒。どちらも 4 スレッド。
同じ (問題, 種) の設定は同じ分担で続けて解き、順は種で回す。見積りは final_20261118 の def の時間 (+ 8 秒・30 秒)。1 分担の目安 1.5 時間。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
F = D.parent / "final_20261118"
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
def times(pat, cap):
    T = defaultdict(list)
    for f in glob.glob(str(pat)):
        for l in open(f):
            r = json.loads(l)
            if r["cfg"] != "def": continue
            T[r["problem"]].append(min(r["result"].get("time") or cap, cap))
    return {k: sum(v) / len(v) for k, v in T.items()}
def order(s, cfgs):
    k = s % len(cfgs); return cfgs[k:] + cfgs[:k]
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -sum(est(p) for p, c in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for (p, s), ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [[p, c, s, tl] for p, c in ts]; load[i] += sum(est(p) for p, c in ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(x) for x in shards), "回", "見積り (時間):", [round(x / 3600, 2) for x in load])
CF = ["def", "epw"]
TP = times(F / "res_P_s*.jsonl", 60)
estP = lambda p: TP.get(p, 60.0) + 8
nP = max(1, round(sum(estP(p) * len(CF) * 3 for p in PROBS) / 5400))
balance({(p, s): [(p, c) for c in order(s, CF)] for p in PROBS for s in (1, 2, 3)}, estP, nP, "shards_P.json", 60)
TM = times(F / "res_M_s*.jsonl", 600)
PM = []
for l in open(D.parent / "prepar_20261107" / "pretime_mitt_t600.jsonl"):
    r = json.loads(l)
    if r.get("probe", 0) + r.get("probe@re", 0) >= 2 and TM.get(r["problem"], 600) < 590:
        PM.append(r["problem"])
estM = lambda p: TM.get(p, 600.0) + 30
nM = max(1, round(sum(estM(p) * len(CF) * 3 for p in PM) / 5400))
balance({(p, s): [(p, c) for c in order(s, CF)] for p in PM for s in (1, 2, 3)}, estM, nM, "shards_M.json", 600)
print("M の問題:", " ".join(sorted(PM)))

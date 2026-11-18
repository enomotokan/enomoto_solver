"""AUDIT の確かめの分担表: python plan.py → shards_A.json・shards_B.json。
A: SUBWAIT が働く 5 問 + unitcal_7 × 種 3〜6 × {def, sw0}、600 秒。B: MIPLIB の 58 問 × 種 1〜3 × {def, rins, qiu, cs2}、60 秒。
同じ (問題, 種) の設定は同じ分担で続けて解き、順は種で回す。見積りは final_20261118 の def の時間 (+ 30 秒・8 秒)、def 以外も同じと置く。
1 分担の目安 1.5 時間。"""
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
TM = times(F / "res_M_s*.jsonl", 600)
PA = ["csched008", "neos-1456979", "neos-2746589-doon", "neos-662469", "ran14x18-disj-8", "unitcal_7"]
estA = lambda p: TM.get(p, 600.0) + 30
nA = max(1, round(sum(estA(p) * 2 * 4 for p in PA) / 5400))
balance({(p, s): [(p, c) for c in order(s, ["def", "sw0"])] for p in PA for s in (3, 4, 5, 6)}, estA, nA, "shards_A.json", 600)
TP = times(F / "res_P_s*.jsonl", 60)
estP = lambda p: TP.get(p, 60.0) + 8
nB = max(1, round(sum(estP(p) * 4 * 3 for p in PROBS) / 5400))
balance({(p, s): [(p, c) for c in order(s, ["def", "rins", "qiu", "cs2"])] for p in PROBS for s in (1, 2, 3)}, estP, nB, "shards_B.json", 60)

"""TREEEFF の分担表を作る。
D (診断): 9 問 × {def, trf, nc, sd, ca, pl, all, highs} × 種 1・2、600 秒。同じ (問題, 種) の回は同じ分担で続けて回す (機械のずれを対で消す)。
使い方: python plan.py D N → shards_D.json (N 分担)"""
import json, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
PROBS_D = ["neos-3627168-kasai", "csched008", "sct2", "supportcase33", "pg5_34", "neos859080", "tr12-30", "traininstance6", "rococoC10-001000"]
# 1 回の見込みの秒 (hiloss の回から。解けない回は 600 + 起動)
EST = {"neos859080": {"highs": 5, "*": 90}, "pg5_34": {"highs": 570}, "supportcase33": {"highs": 250}, "neos-3627168-kasai": {"highs": 140},
       "csched008": {"highs": 320}, "tr12-30": {"highs": 460}, "sct2": {"highs": 560}}
def est(p, c):
    e = EST.get(p, {})
    return e.get(c, e.get("*", 625))
def balance(tasks, n, out, tl):
    groups = defaultdict(list)
    for t in tasks: groups[(t[0], t[2])].append(t)
    gl = sorted(groups.items(), key=lambda kv: -sum(est(t[0], t[1]) for t in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for _, ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += sum(est(t[0], t[1]) for t in ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(tasks), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜", max(round(x / 3600, 2) for x in load))
if sys.argv[1] == "D":
    cfgs = ["def", "trf", "nc", "sd", "ca", "pl", "all", "highs"]
    balance([(p, c, s) for p in PROBS_D for s in (1, 2) for c in cfgs], int(sys.argv[2]), "shards_D.json", 600)

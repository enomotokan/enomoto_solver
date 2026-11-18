"""分担表: python plan.py NP NM → shards_P.json (58 問 × 種 1〜3 × 設定 CF_P、4 スレッド・60 秒) と
shards_M.json (重い問題 MPROBS × 種 1・2 × 設定 CF_M、4 スレッド・600 秒)。同じ (問題, 種) の全設定は同じ分担で続けて解く。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
CF_P = ["def", "t01", "t03", "t05", "s00", "sm03", "s00c"]
CF_M = ["def", "t01", "t03", "t05", "s00", "sm03", "s00c"]
MPROBS = ["mzzv11", "neos-933966", "supportcase33", "neos-3216931-puriri", "neos-1456979", "neos-3627168-kasai", "irp"]
T = defaultdict(list)
for f in glob.glob(str(D.parent / "cand1118_default_20261116/res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "def":
            T[r["problem"]].append(min(r["result"].get("time") or 60, 60))
def balance(items, est, n, out):
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for key, runs in sorted(items, key=lambda kv: -est(kv[0])):
        i = min(range(n), key=lambda j: load[j]); shards[i] += runs; load[i] += est(key)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, n, "分担、", sum(len(s) for s in shards), "回、見積り (時間)", [round(x / 3600, 2) for x in load])
NP, NM = int(sys.argv[1]), int(sys.argv[2])
estP = lambda k: len(CF_P) * ((sum(T[k[0]]) / len(T[k[0]]) if T[k[0]] else 60) + 8)
balance([((p, s), [[p, c, s, 60] for c in CF_P]) for p in PROBS for s in (1, 2, 3)], estP, NP, "shards_P.json")
balance([((p, s), [[p, c, s, 600] for c in CF_M]) for p in MPROBS for s in (1, 2)], lambda k: len(CF_M) * 620, NM, "shards_M.json")

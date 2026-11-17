"""分担表: python plan.py NP NM → shards_P.json (58 問 × 種 1〜3 × {def, aff}、60 秒) と shards_M.json (M の問題 × 種 1・2 × {def, aff}、600 秒)。
shards_M.json (Mittelmann の重い問題 6 問 × 種 1・2 × {def, inc2}、4 スレッド・600 秒、24 回)。同じ問題・種の 2 設定は同じ分担で続けて解く。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
MPROBS = ["tbfp-network", "irp", "mzzv11", "neos-3627168-kasai", "rococoC10-001000", "supportcase33"]
T = defaultdict(list)
for f in glob.glob(str(D.parent / "cand1118_default_20261116/res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "def": T[r["problem"]].append(min(r["result"].get("time") or 60, 60))
def balance(items, est, n, out):
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for key, runs in sorted(items, key=lambda kv: -est(kv[0])):
        i = min(range(n), key=lambda j: load[j]); shards[i] += runs; load[i] += est(key)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, n, "分担、", sum(len(s) for s in shards), "回、見積り (時間)", [round(x / 3600, 2) for x in load])
NP, NM = int(sys.argv[1]), int(sys.argv[2])
estP = lambda p: 6 * ((sum(T[p]) / len(T[p]) if T[p] else 60) + 8)
balance([(p, [[p, c, s, 60] for s in (1, 2, 3) for c in ("def", "aff")]) for p in PROBS], estP, NP, "shards_P.json")
balance([((p, s), [[p, c, s, 600] for c in ("def", "aff")]) for p in MPROBS for s in (1, 2)], lambda k: 2 * 620, NM, "shards_M.json")

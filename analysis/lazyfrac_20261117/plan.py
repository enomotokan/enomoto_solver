"""分担表: python plan.py NP NM → shards_P.json (58 問 × 種 4〜6 × 設定 CF、4 スレッド・60 秒) と
shards_M.json (重い問題 MPROBS × 種 3・4 × 設定 CF、4 スレッド・600 秒)。同じ (問題, 種) の全設定は同じ分担で続けて解く。
種 1〜3 (P) と種 1・2 (M) は門のしきい値を選ぶのに使ったので使わない。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
CF = ["def", "gs", "cs", "g1"]
# genlazy の M の 7 問 + Mittelmann 186 問のうち外せる割合 f の大きい問題 (count_f.jsonl、README の表)
MPROBS = ["mzzv11", "neos-933966", "supportcase33", "neos-3216931-puriri", "neos-1456979", "neos-3627168-kasai", "irp"]
MPROBS += json.loads((D / "mprobs_extra.json").read_text()) if (D / "mprobs_extra.json").exists() else []
T = defaultdict(list)
for f in glob.glob(str(D.parent / "cand1118_default_20261116/res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "def":
            T[r["problem"]].append(min(r["result"].get("time") or 60, 60))
# M の見積り: def の秒 (vshighs_20261117 の Mittelmann 600 秒・genlazy の M、時間切れは 600) の平均 × 1.3 + 20 を 4 設定分
TM = defaultdict(list)
for f in glob.glob(str(D.parent / "vshighs_20261117/res_[MA]_s*.jsonl")) + glob.glob(str(D.parent / "genlazy_20261117/res_M_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "def":
            ok = str(r["result"].get("status")).lower() in ("optimal", "infeasible")
            TM[r["problem"]].append(min(r["result"].get("time") or 600, 600) if ok else 600)
estM = lambda k: len(CF) * (min(600, 1.3 * (sum(TM[k[0]]) / len(TM[k[0]]) if TM[k[0]] else 600)) + 20)
def balance(items, est, n, out):
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for key, runs in sorted(items, key=lambda kv: -est(kv[0])):
        i = min(range(n), key=lambda j: load[j]); shards[i] += runs; load[i] += est(key)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, n, "分担、", sum(len(s) for s in shards), "回、見積り (時間)", [round(x / 3600, 2) for x in load])
NP, NM = int(sys.argv[1]), int(sys.argv[2])
estP = lambda k: len(CF) * ((sum(T[k[0]]) / len(T[k[0]]) if T[k[0]] else 60) + 8)
balance([((p, s), [[p, c, s, 60] for c in CF]) for p in PROBS for s in (4, 5, 6)], estP, NP, "shards_P.json")
balance([((p, s), [[p, c, s, 600] for c in CF]) for p in MPROBS for s in (3, 4)], estM, NM, "shards_M.json")

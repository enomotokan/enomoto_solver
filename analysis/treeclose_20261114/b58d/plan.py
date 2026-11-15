"""TREECLOSE (analysis/treeclose_20261114.md): cand1114 (base) と dfs (b58d/cfgs.py: 束 cand1115 + 深さ優先寄り + TC=as + 基底の退化した列の固定) を 58 問 (analysis/implint_tu_20261110/probs_all.py の P58) x 種 1〜2 で対にして測る分担表 shards_dfs58.json を作る。
60 秒・4 スレッド。時間の見積りは drcr_20261110/b58*.jsonl の既定の回 (解けなければ 60 秒) + 15 秒。同じ (問題, 種) の base と rch は同じ分担に入れる。
使い方: python plan.py NS (回すのは python run_shard.py K dfs58)"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parents[1] / "implint_tu_20261110"))
from probs_all import P58  # noqa: E402
NS = int(sys.argv[1]); TL = 60
T = defaultdict(list)
for f in glob.glob(str(D.parents[1] / "drcr_20261110" / "b58*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r.get("cfg") != "def":
            continue
        x = r["result"]; st = (x.get("status") or "").lower(); t = x.get("time") or TL
        T[r["problem"]].append(t if st in ("optimal", "infeasible") and t <= TL + 5 else TL)
est = {p: (sum(T[p]) / len(T[p]) if T[p] else TL) + 15 for p in P58}
pairs = sorted(((p, s) for p in P58 for s in (1, 2)), key=lambda t: -est[t[0]])
load_ = [0.0] * NS; shards = [[] for _ in range(NS)]
for p, s in pairs:
    i = min(range(NS), key=lambda j: load_[j])
    shards[i] += [[p, "base", s], [p, "dfs", s]]; load_[i] += 2 * est[p]
(D / "shards_dfs58.json").write_text(json.dumps({"tl": TL, "shards": shards, "est_hours": [round(x / 3600, 2) for x in load_]}, indent=0))
print(len(P58), "問", 4 * len(P58), "回", "分担ごとの見積り (時間):", [round(x / 3600, 2) for x in load_])

"""分担表を作る: python plan.py N [--loo 文字,... --probs 問題,...] → shards.json (--loo なら shards_loo.json)。58 問 (analysis/cbcimpl_20261029/probs.py) × 種 1〜4 × cfg.py の CFGS。
--loo m,p: base・C と C-m・C-p だけを、--probs の問題 (既定 58 問) で。
同じ (問題, 種) の全設定を同じ分担に入れる (時期・機械のずれを対で消す)。時間の見積りは cand1114 の P の回 (wall)。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[2]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029")); sys.path.insert(0, str(D))
from probs import PROBS  # noqa: E402
from cfg import CFGS, CFGS_ALL  # noqa: E402
N = int(sys.argv[1]); SEEDS = [1, 2, 3, 4]
a = sys.argv[2:]; loo = a[a.index("--loo") + 1].split(",") if "--loo" in a else None
if "--probs" in a: PROBS = a[a.index("--probs") + 1].split(",")
cfgs = ["base", "C"] + [f"C-{x}" for x in loo] if loo else list(CFGS)
assert all(c in CFGS_ALL for c in cfgs)
name = "shards_loo.json" if loo else "shards.json"
est = defaultdict(lambda: 60.0)
for f in glob.glob(str(WT / "analysis/cand1114_20261113/res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "cand1114": est[r["problem"], r["seed"]] = r.get("wall", 60.0)
groups = sorted(((p, s) for p in PROBS for s in SEEDS), key=lambda g: -est[g])
load = [0.0] * N; shards = [[] for _ in range(N)]
for p, s in groups:
    i = min(range(N), key=lambda j: load[j])
    shards[i] += [[p, c, s, 60] for c in cfgs]; load[i] += est[p, s] * len(cfgs)
(D / name).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print(len(groups), "組", sum(len(x) for x in shards), "回、見積り (時間):", [round(x / 3600, 2) for x in load])

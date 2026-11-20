"""Mittelmann の残り 138 問 (mitt_parts.json の 186 問のうち M の 48 問以外) の分担表: python planA.py N → shards_A.json
× 種 1 × {def, highs}、600 秒。見積り: HiGHS の 60 秒の回 (analysis/highs_mitt_20261103) で解けた問題はその時間、ほかは 600 秒 (+30 秒)。"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
N = int(sys.argv[1])
allp = sorted({x for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for x in part})
M = {p for s in json.loads((D / "shards_M.json").read_text())["shards"] for p, *_ in s}
H = {}
for f in glob.glob(str(WT / "analysis/highs_mitt_20261103/highs_s*.jsonl")):
    for l in open(f):
        r = json.loads(l); H[r["problem"]] = r["result"]
def solved(r): return str(r.get("status", "")).lower() in ("optimal", "infeasible")
def est(p): return (H[p].get("time", 60) if p in H and solved(H[p]) else 600) + 30
rest = [p for p in allp if p not in M]
groups = sorted(rest, key=lambda p: -est(p))
load = [0.0] * N; shards = [[] for _ in range(N)]
for p in groups:
    i = min(range(N), key=lambda j: load[j]); shards[i] += [[p, "def", 1, 600], [p, "highs", 1, 600]]; load[i] += 2 * est(p)
(D / "shards_A.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print(len(rest), "問", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜", max(round(x / 3600, 2) for x in load))

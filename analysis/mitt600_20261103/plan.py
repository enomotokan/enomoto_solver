"""Mittelmann 186 問を 600 秒で ENOMOTO (今の既定) と HiGHS で比べる計測の分担表を作る (2026-11-03)。
60 秒の結果 (ENOMOTO: bench21 の def・回 115、HiGHS: analysis/highs_mitt_20261103) から 1 問の見込みの時間 (解けたらその時間、解けなければ 600 秒) を出し、
見込みの和が近くなるように N 分担に分ける (大きい方から、いちばん軽い分担へ)。使い方: python plan.py N > shards.json"""
import json, sys, glob
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
n = int(sys.argv[1])
probs = sorted({p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part})
def est(r):
    return r["time"] if r.get("status") in ("optimal", "infeasible") and r.get("time", 99) <= 60 else 600.0
eo, eh = {}, {}
for f in glob.glob(str(WT / "analysis/halve_submip_20261030/bench21_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "def" and r["rep"] == 115: eo[r["problem"]] = est(r["result"])
for f in glob.glob(str(WT / "analysis/highs_mitt_20261103/highs_s*.jsonl")):
    for l in open(f):
        r = json.loads(l); eh[r["problem"]] = est(r["result"])
cost = {p: eo.get(p, 600.0) + eh.get(p, 600.0) + 20 for p in probs}
shards = [[] for _ in range(n)]; load = [0.0] * n
for p in sorted(probs, key=lambda p: -cost[p]):
    i = min(range(n), key=lambda i: load[i]); shards[i].append(p); load[i] += cost[p]
json.dump({"n": n, "load_s": [round(x) for x in load], "shards": shards}, sys.stdout, ensure_ascii=False)

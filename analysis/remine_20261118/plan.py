"""remine の短い計測の分担表: python plan.py SET N → shards_SET.json
- G (README の 5、GAP4 の既定 rocn の害の確かめ): 9 問 × 種 1〜3 × {def, g4off}、4 スレッド・60 秒 (54 回)。
- L (README の 3、LUCLOCK=g を小さい問題だけ): P の 58 問のうち列 ≤ 1500 の 32 問 + 対照の大きい 2 問 (swath1・nw04) × 種 1〜4 × {def, lucg}、
  4 スレッド・60 秒 (272 回)。
同じ (問題, 種) の設定は同じ分担で続けて解き、順は種でずらす。見積り: vshighs_20261117 の def の時間の平均 + 8 秒。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
SET, N = sys.argv[1], int(sys.argv[2])
if SET == "G":
    PROBS = ["drayage-100-23", "drayage-25-23", "neos-960392", "neos-860300", "ns1116954", "physiciansched6-2", "mas76", "binkar10_1", "nw04"]
    CS, SEEDS = ["def", "g4off"], (1, 2, 3)
elif SET == "L":
    K = json.loads((D.parent / "binheur_20261111/kinds.json").read_text())
    P58 = {json.loads(l)["problem"] for f in glob.glob(str(D.parent / "vshighs_20261117/res_P_s*.jsonl")) for l in open(f)}
    PROBS = sorted(p for p in P58 if p in K and K[p]["n"] <= 1500) + ["swath1", "nw04"]
    CS, SEEDS = ["def", "lucg"], (1, 2, 3, 4)
else:
    sys.exit("SET は G か L")
T = defaultdict(list)
for f in glob.glob(str(D.parent / "vshighs_20261117/res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "def": T[r["problem"]].append(min(r["result"].get("time") or 60, 60))
est = {p: sum(T[p]) / len(T[p]) + 8 if T[p] else 68 for p in PROBS}
groups = {(p, s): [(p, CS[(i + s) % len(CS)], s) for i in range(len(CS))] for p in PROBS for s in SEEDS}
gl = sorted(groups.items(), key=lambda kv: -est[kv[0][0]])
load = [0.0] * N; shards = [[] for _ in range(N)]
for (p, s), ts in gl:
    i = min(range(N), key=lambda j: load[j]); shards[i] += [list(t) + [60] for t in ts]; load[i] += est[p] * len(ts)
(D / f"shards_{SET}.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print(SET, len(PROBS), "問", len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", [round(x / 3600, 2) for x in load])

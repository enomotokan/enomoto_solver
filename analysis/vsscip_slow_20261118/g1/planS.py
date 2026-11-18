"""群 1 の直した版 (ORBPP の列の数の下限 10 = ENOMOTO_T_MIP_ORBPP_MIN_COLS の既定) の短い比べ: python planS.py N → shards_S.json
M の 4 問 (CMS750_4・rococoC10-001000・timtab1・neos-787933、600 秒) と P の 2 問 (neos-911970・graph20-20-1rand、60 秒) × 種 1〜3 × {def, g1}。
見積り: g1 の M・P の結果 (両設定の平均、時間切れは上限) + 30 秒・8 秒。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
N = int(sys.argv[1]) if len(sys.argv) > 1 else 1
T = defaultdict(list)
for f in glob.glob(str(D / "res_[MP]_s*.jsonl")):
    for l in open(f):
        r = json.loads(l); ok = str(r["result"].get("status")).lower() in ("optimal", "infeasible")
        T[r["problem"]].append(min(r["result"]["time"], r["tl"]) if ok else r["tl"])
PROBS = [("CMS750_4", 600), ("rococoC10-001000", 600), ("timtab1", 600), ("neos-787933", 600), ("neos-911970", 60), ("graph20-20-1rand", 60)]
groups = [(p, s, tl) for p, tl in PROBS for s in (1, 2, 3)]
est = lambda p, tl: min(sum(T[p]) / len(T[p]), tl) + (30 if tl == 600 else 8)
groups.sort(key=lambda g: -est(g[0], g[2]))
load = [0.0] * N; shards = [[] for _ in range(N)]
for p, s, tl in groups:
    i = min(range(N), key=lambda j: load[j]); cs = ["def", "g1"] if s % 2 else ["g1", "def"]
    shards[i] += [[p, c, s, tl] for c in cs]; load[i] += 2 * est(p, tl)
(D / "shards_S.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print("shards_S.json", len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", [round(x / 3600, 2) for x in load])

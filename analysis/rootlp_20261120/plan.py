"""ROOTLP の分担表: python plan.py N → shards_F.json
F: 目的が全て 0 の問題 (P・M の 7 問 + MIPLIB の lectsched-1・pythago7824・neos-578379・supportcase3) × 種 1〜3 ×
{def, ncnr, def_t1, ncnr_t1}、600 秒。同じ (問題, 種, スレッド数) の 2 組は同じ分担で続けて解く。見積りは手元と hang の F の時間 (解けない問題は 600 秒)。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
N = int(sys.argv[1]) if len(sys.argv) > 1 else 6
EST = {"cryptanalysiskb128n5obj14": 600, "cryptanalysiskb128n5obj16": 600, "fhnw-binpack4-4": 250, "fhnw-binpack4-48": 30,
       "neos-3004026-krka": 20, "ns1116954": 40, "ns1952667": 600, "lectsched-1": 100, "pythago7824": 600, "neos-578379": 600, "supportcase3": 600}
groups = []
for p, e in EST.items():
    for s in (1, 2, 3):
        for th in ("", "_t1"):
            k = s % 2
            cs = [f"def{th}", f"ncnr{th}"]
            groups.append((2 * e + 20, [[p, c, s, 600] for c in cs[k:] + cs[:k]]))
groups.sort(key=lambda g: -g[0])
load = [0.0] * N; shards = [[] for _ in range(N)]
for e, runs in groups:
    i = min(range(N), key=lambda j: load[j]); shards[i] += runs; load[i] += e
(D / "shards_F.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print("shards_F.json", sum(len(s) for s in shards), "回", "見積り (時間):", [round(x / 3600, 2) for x in load])

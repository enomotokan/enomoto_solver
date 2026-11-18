"""短い計測 S の分担表: python planS.py N → shards_S.json
効く問題と悪化した問題に絞り、試作ごとに {def, 無条件, 条件付き} を同じ (問題, 種) で続けて解く (同じ機械)。
- sbt: 一般整数 16 問 (analysis/cbcimpl_20261029/summ6.py の GEN) + neos-911970・10teams × {def, sbt, sbtc}
- SUBFIX (60 秒): neos-960392・10teams・neos-827175・noswot・sp150x300d × {def, sf0, sfcap}
- SUBFIX (600 秒): neos-873061・neos859080・fiball・supportcase33・mzzv42z・pg5_34 × {def, sf0, sfcap} (種 1・2)
- rb: 10teams・noswot と rootstop_20261111.md で動いた問題 × {def, rb}
60 秒の問題は種 1〜4、600 秒の問題は種 1・2。問題ごとに設定の和集合を回す (def は 1 回)。順は種で回す。
見積り: vshighs_20261117 の def の時間 (+ 8 秒・30 秒)。解けなかった回は上限で数える。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
A = D.parents[0]
N = int(sys.argv[1])
GEN = ["neos-3083819-nubu", "roll3000", "mzzv42z", "neos-1456979", "mik-250-20-75-4", "gt2", "qnet1", "timtab1", "30n20b8", "gen-ip002",
       "nu25-pr12", "gen-ip054", "blend2", "noswot", "rout", "flugpl"]
P = defaultdict(set)
for p in GEN + ["neos-911970", "10teams"]:
    P[p] |= {"def", "sbt", "sbtc"}
for p in ["neos-960392", "10teams", "neos-827175", "noswot", "sp150x300d"]:
    P[p] |= {"def", "sf0", "sfcap"}
for p in ["10teams", "noswot", "mc11", "neos5", "neos-911970", "mik-250-20-75-4", "swath1", "fiber", "p200x1188c", "timtab1", "eil33-2", "dcmulti"]:
    P[p] |= {"def", "rb"}
M = {p: {"def", "sf0", "sfcap"} for p in ["neos-873061", "neos859080", "fiball", "supportcase33", "mzzv42z", "pg5_34"]}
ORDER = ["def", "sbt", "sbtc", "sf0", "sfcap", "rb"]
def solved(r): return str(r["result"].get("status", "")).lower() in ("optimal", "infeasible")
def times(pat, cap):
    T = defaultdict(list)
    for f in glob.glob(str(pat)):
        for l in open(f):
            r = json.loads(l)
            if r["cfg"] != "def": continue
            T[r["problem"]].append(min(r["result"]["time"], cap) if solved(r) else cap)
    return {k: sum(v) / len(v) for k, v in T.items()}
TP = times(A / "vshighs_20261117/res_P_s*.jsonl", 60)
TM = times(A / "vshighs_20261117/res_M_s*.jsonl", 600)
groups = {}
for p, cs in P.items():
    for s in (1, 2, 3, 4):
        c = [x for x in ORDER if x in cs]; k = s % len(c)
        groups[(p, s)] = ([(p, x, s, 60) for x in c[k:] + c[:k]], (TP.get(p, 60.0) + 8) * len(c))
for p, cs in M.items():
    for s in (1, 2):
        c = [x for x in ORDER if x in cs]; k = s % len(c)
        groups[(p, s)] = ([(p, x, s, 600) for x in c[k:] + c[:k]], (TM.get(p, 600.0) + 30) * len(c))
gl = sorted(groups.values(), key=lambda g: -g[1])
load = [0.0] * N; shards = [[] for _ in range(N)]
for ts, e in gl:
    i = min(range(N), key=lambda j: load[j]); shards[i] += [list(t) for t in ts]; load[i] += e
(D / "shards_S.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print("shards_S.json", len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜", max(round(x / 3600, 2) for x in load), "計", round(sum(load) / 3600, 1))

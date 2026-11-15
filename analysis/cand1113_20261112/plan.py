"""cand1113 の確かめの分担表を作る。(M) Mittelmann 186 問 × 種 1・2 × {def, cand1113} 600 秒 + 種 1 の HiGHS、(P) 58 問 (MIPLIB) × 種 1〜4 × {def, cand1113}
60 秒 + physiciansched6-2 の種 5〜12 (OBJINT の確かめで揺れが大きかったため)。M は同じ問題の回を全部、P は同じ (問題, 種) の def と cand1113 を同じ分担に入れる
(時期・機械のずれを対で消す)。時間の見積りは res9 (M) と cand1112 の P の回。使い方: python plan.py NM NP → shards_M.json・shards_P.json"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
NM, NP = int(sys.argv[1]), int(sys.argv[2])
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -sum(est[t[0]] for t in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for _, ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += sum(est[t[0]] for t in ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜", max(round(x / 3600, 2) for x in load))
M = json.loads((WT / "analysis/mitt600_20261103/shards9.json").read_text())["problems"]
T = defaultdict(list)
for f in glob.glob(str(WT / "analysis/mitt600_20261103/res9_s*.jsonl")):
    for l in open(f):
        r = json.loads(l); x = r["result"]; t = x.get("time") or 600
        T[r["problem"]].append(min(t, 600))
estM = {p: (sum(T[p]) / len(T[p]) if T[p] else 600) + 30 for p in M}
gM = {p: [(p, "def", 1), (p, "cand1113", 1), (p, "highs", 1), (p, "def", 2), (p, "cand1113", 2)] for p in M}
balance(gM, estM, NM, "shards_M.json", 600)
T2 = defaultdict(list)
for f in glob.glob(str(WT / "analysis/cand1112_20261111/res_P_s*.jsonl")):
    for l in open(f):
        try: r = json.loads(l)
        except Exception: continue
        p = r.get("problem"); x = r.get("result", r); t = x.get("time") if isinstance(x, dict) else None
        if p and isinstance(t, (int, float)): T2[p].append(min(t, 60))
estP = {p: (sum(T2[p]) / len(T2[p]) if T2[p] else 40) + 8 for p in PROBS}
gP = {}
for p in PROBS:
    seeds = (1, 2, 3, 4) + ((5, 6, 7, 8, 9, 10, 11, 12) if p.startswith("physiciansched6-2") else ())
    for s in seeds:
        gP[(p, s)] = [(p, "def", s), (p, "cand1113", s)]
balance(gP, estP, NP, "shards_P.json", 60)

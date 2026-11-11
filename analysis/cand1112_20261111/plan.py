"""cand1112 の確かめの分担表を作る。2 つの組: (M) Mittelmann 186 問 × 種 1・2、600 秒、(P) 58 問 (MIPLIB) × 種 1〜4、60 秒。
どちらも同じ (問題, 種) の def と cand1112 を同じ分担で続けて回す (時期・機械のずれを対で消す)。時間の見積りは res9 (M) と既定の 60 秒の回 (P)。
使い方: python plan.py NM NP  → shards_M.json (NM 分担)・shards_P.json (NP 分担)"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
NM, NP = int(sys.argv[1]), int(sys.argv[2])
def balance(tasks, est, n, out, tl):
    groups = defaultdict(list)
    for t in tasks: groups[(t[0], t[2])].append(t)
    gl = sorted(groups.items(), key=lambda kv: -2 * est[kv[0][0]])
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for (p, s), ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += 2 * est[p]
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(tasks), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜", max(round(x / 3600, 2) for x in load))
M = json.loads((WT / "analysis/mitt600_20261103/shards9.json").read_text())["problems"]
T = defaultdict(list)
for f in glob.glob(str(WT / "analysis/mitt600_20261103/res9_s*.jsonl")):
    for l in open(f):
        r = json.loads(l); x = r["result"]; t = x.get("time") or 600
        T[r["problem"]].append(min(t, 600))
estM = {p: (sum(T[p]) / len(T[p]) if T[p] else 600) + 30 for p in M}
balance([(p, c, s) for p in M for s in (1, 2) for c in ("def", "cand1112")], estM, NM, "shards_M.json", 600)
T2 = defaultdict(list)
for f in glob.glob(str(WT / "analysis/cand1111_20261111/res_P_s*.jsonl")) + glob.glob(str(WT / "analysis/rcstrong_20261111/*.jsonl")):
    for l in open(f):
        try: r = json.loads(l)
        except Exception: continue
        p = r.get("problem"); x = r.get("result", r); t = x.get("time") if isinstance(x, dict) else None
        if p and isinstance(t, (int, float)): T2[p].append(min(t, 60))
estP = {p: (sum(T2[p]) / len(T2[p]) if T2[p] else 40) + 8 for p in PROBS}
balance([(p, c, s) for p in PROBS for s in (1, 2, 3, 4) for c in ("def", "cand1112")], estP, NP, "shards_P.json", 60)

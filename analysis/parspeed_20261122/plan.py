"""parspeed の分担表: python plan.py [上限の時間 (既定 1.8)] → shards_P.json
P (MIPLIB の 58 問) のうち、analysis/vsscip_20261120 の 1 スレッド (def_t1) が種 1〜3 の中央値で 3 秒以上かかり、
4 スレッド (def) か 1 スレッドで 60 秒以内に解けた回がある問題 (PROBS)。× 種 1〜3 × {t1, t4, t1c, t4c}、120 秒。
見積り: vsscip の 1・4 スレッドの時間 (解けなければ 120 秒) + 1 回ごとに 8 秒 (cutoff ありも同じ時間で見積もる)。
同じ (問題, 種) の 4 組は同じ分担で続けて解き、順は組ごとに回す。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
OLD = D.parent / "vsscip_20261120"
CAP = float(sys.argv[1]) if len(sys.argv) > 1 else 1.8
TL = 120
CF = ["t1", "t4", "t1c", "t4c"]
T = {}
for f in OLD.glob("res_P_s*.jsonl"):
    for l in open(f):
        r = json.loads(l); st = str(r["result"].get("status", "")).lower()
        T[(r["problem"], r["cfg"], r["seed"])] = r["result"]["time"] if st in ("optimal", "infeasible") else None
probs = sorted({p for p, c, s in T})
PROBS = []
for p in probs:
    t1 = sorted((T.get((p, "def_t1", s)) or 999) for s in (1, 2, 3))
    any_solved = any(T.get((p, c, s)) is not None and T[(p, c, s)] <= 60 for c in ("def", "def_t1") for s in (1, 2, 3))
    if t1[1] >= 3 and any_solved: PROBS.append(p)
def est(p, c, s):
    t = T.get((p, "def_t1" if c.startswith("t1") else "def", s))
    return (TL if t is None else min(t, TL)) + 8
groups = {(p, s): [(p, CF[(i + k) % 4], s) for k in range(4)] for i, (p, s) in enumerate((p, s) for p in PROBS for s in (1, 2, 3))}
gl = sorted(groups.items(), key=lambda kv: -sum(est(*t) for t in kv[1]))
n = 1
while True:
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for _, ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [TL] for t in ts]; load[i] += sum(est(*t) for t in ts)
    if max(load) <= CAP * 3600: break
    n += 1
(D / "shards_P.json").write_text(json.dumps({"probs": PROBS, "shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print(len(PROBS), "問", PROBS)
print("shards_P.json", sum(map(len, shards)), "回、分担", n, "見積り (時間):", round(min(load) / 3600, 2), "〜", round(max(load) / 3600, 2), "計", round(sum(load) / 3600, 1))

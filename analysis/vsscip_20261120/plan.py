"""SCIP との比べの測り直しの分担表 (2026-11-20): python plan.py [上限の時間 (既定 1.8。2 時間に余裕を残す)] → shards_M.json・shards_P.json
M: Mittelmann 186 問 × 種 1 × {def, def_t1}、600 秒。P: MIPLIB の 58 問 × 種 1〜3 × {def, def_t1}、60 秒 (analysis/vsscip_20261117 と同じ問題・種)。
SCIP は回さない (11/17 の結果を summ.py で使い回す)。同じ (問題, 種) の 2 組は同じ分担で続けて解き、順は組ごとに入れ替える。
見積り: 11/17 の同じ設定の時間 (解けなければ上限) + 1 回ごとに M 30 秒・P 8 秒。分担の数は、見積りの最大が上限の時間に収まる最小の数。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
OLD = D.parent / "vsscip_20261117"
CAP = float(sys.argv[1]) if len(sys.argv) > 1 else 1.8
CF = ["def", "def_t1"]
def solved(r): return str(r["result"].get("status", "")).lower() in ("optimal", "infeasible")
def load(S, tl):
    T = {}
    for f in OLD.glob(f"res_{S}_s*.jsonl"):
        for l in open(f):
            if l.strip():
                r = json.loads(l)
                if r["cfg"] in CF: T[(r["problem"], r["cfg"], r["seed"])] = min(r["result"]["time"], tl) if solved(r) else tl
    return T
def balance(groups, est, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -sum(est(*t) for t in kv[1]))
    n = 1
    while True:
        load_ = [0.0] * n; shards = [[] for _ in range(n)]
        for _, ts in gl:
            i = min(range(n), key=lambda j: load_[j]); shards[i] += [list(t) + [tl] for t in ts]; load_[i] += sum(est(*t) for t in ts)
        if max(load_) <= CAP * 3600: break
        n += 1
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load_]}, indent=0))
    print(out, len(groups), "組", sum(map(len, shards)), "回、分担", n, "見積り (時間):", round(min(load_) / 3600, 2), "〜", round(max(load_) / 3600, 2), "計", round(sum(load_) / 3600, 1))
def order(p, s, i): return [(p, c, s) for c in (CF if i % 2 == 0 else CF[::-1])]
for S, tl, pad in (("M", 600, 30), ("P", 60, 8)):
    T = load(S, tl)
    keys = sorted({(p, s) for p, c, s in T})
    balance({(p, s): order(p, s, i) for i, (p, s) in enumerate(keys)}, lambda p, c, s: T.get((p, c, s), tl) + pad, f"shards_{S}.json", tl)

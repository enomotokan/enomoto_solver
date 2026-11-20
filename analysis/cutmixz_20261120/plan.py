"""CUTMIX=z の広い計測の分担表: python plan.py [上限の時間 (既定 1.75)] → shards_M.json・shards_P.json
M: Mittelmann 186 問 (analysis/vsscip_20261120 の M) のうち z が働く問題 (zscan.jsonl の intsmall ≥ 0.5、根に届かず不明のものも入れる)
   × 種 1・2 × {def, z}、600 秒。働かない問題は def と同じ道筋になるので回さない。
P: MIPLIB の 58 問 × 種 1〜3 × {def, z}、60 秒 (働かない問題も入れる。集計で働く・働かないを分けて見る)。
同じ (問題, 種) の 2 組は同じ分担で続けて解き、順は組ごとに入れ替える。見積り: analysis/vsscip_20261117 の def の時間 (種 1。解けなければ上限)
+ 1 回ごとに M 30 秒・P 8 秒。分担の数は、見積りの最大が上限の時間に収まる最小の数。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
OLD = D.parent / "vsscip_20261117"
CAP = float(sys.argv[1]) if len(sys.argv) > 1 else 1.75
CF = ["def", "z"]
ZMIN = 0.5
def solved(r): return str(r["result"].get("status", "")).lower() in ("optimal", "infeasible")
def load(S, tl):
    T = {}
    for f in OLD.glob(f"res_{S}_s*.jsonl"):
        for l in open(f):
            if l.strip():
                r = json.loads(l)
                if r["cfg"] == "def": T[(r["problem"], r["seed"])] = min(r["result"]["time"], tl) if solved(r) else tl
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
Z = {json.loads(l)["problem"]: json.loads(l)["intsmall"] for l in open(D / "zscan.jsonl")}
for S, tl, pad, seeds in (("M", 600, 30, (1, 2)), ("P", 60, 8, (1, 2, 3))):
    T = load(S, tl)
    probs = sorted({p for p, s in T})
    if S == "M":
        miss = [p for p in probs if p not in Z]
        assert not miss, f"zscan にない: {miss}"
        probs = [p for p in probs if Z[p] is None or Z[p] >= ZMIN]
        print("M の z が働く問題", len(probs), "(不明", sum(Z[p] is None for p in probs), ")")
    keys = [(p, s) for p in probs for s in seeds]
    est = lambda p, c, s: T.get((p, s), T.get((p, 1), tl)) + pad
    balance({(p, s): order(p, s, i) for i, (p, s) in enumerate(keys)}, est, f"shards_{S}.json", tl)

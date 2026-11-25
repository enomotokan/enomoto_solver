"""分担表: python plan.py [分担の数 (既定 8)] → shards.json (P = MIPLIB の 58 問 × 種 1〜3 × cfg.py の全設定、60 秒)。
同じ (問題, 種) の設定 (既定 2 つと試作 4) は同じ分担で続けて解き、順は組ごとに回して入れ替える。
見積り (1 回、秒): analysis/final_20261118 の P の def (4 スレッド)・def_t1 の同じ (問題, 種) の時間 (解けなければ 60)、試作も同じとみる、+ 待ち 8 秒。"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D)); sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from cfg import CFGS  # noqa: E402
from probs import PROBS  # noqa: E402
NS = int(sys.argv[1]) if len(sys.argv) > 1 else 8
TL = 60
def solved(r): return str(r["result"].get("status", "")).lower() in ("optimal", "infeasible")
T = {}
for f in glob.glob(str(D.parent / "final_20261118" / "res_P_s*.jsonl")):
    for l in open(f):
        if l.strip():
            r = json.loads(l)
            if r["cfg"] in ("def", "def_t1"):
                T[(r["problem"], "t1" if r["cfg"] == "def_t1" else "t4", r["seed"])] = min(r["result"]["time"], TL) if solved(r) else TL
def est(p, c, s): return T.get((p, "t1" if c.endswith("_t1") else "t4", s), TL) + 8
CF = list(CFGS)
groups = {(p, s): [(p, c, s) for c in CF[(i + s) % len(CF):] + CF[:(i + s) % len(CF)]] for i, p in enumerate(PROBS) for s in (1, 2, 3)}
gl = sorted(groups.items(), key=lambda kv: -sum(est(*t) for t in kv[1]))
load = [0.0] * NS; shards = [[] for _ in range(NS)]
for _, ts in gl:
    i = min(range(NS), key=lambda j: load[j]); shards[i] += [list(t) + [TL] for t in ts]; load[i] += sum(est(*t) for t in ts)
(D / "shards.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print(f"shards.json: {len(PROBS)} 問 × 種 3 × {len(CF)} 設定 = {sum(len(s) for s in shards)} 回、{NS} 分担、見積り (時間) 各 {min(load)/3600:.2f}〜{max(load)/3600:.2f}、計 {sum(load)/3600:.1f}")

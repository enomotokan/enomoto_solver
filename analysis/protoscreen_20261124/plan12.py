"""12 番目の試作 (cutu = ENOMOTO_MIP_CUTOFF_USER_TREE=1、analysis/ti6_20261124) の分担表: python plan12.py [分担の数 (既定 2)] → shards_c12.json。
P の 58 問 × 種 1〜3 × {cutu_t1, cutu_t4}、60 秒。既定は shards.json の def_t1・def_t4 の行を共有する (同じ分担ではないので、組ではなく共有)。
回し方: python run.py K shards_c12.json res_c12_sK.jsonl (K = 0〜)。見積りは plan.py と同じ。"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
NS = int(sys.argv[1]) if len(sys.argv) > 1 else 2
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
CF = ["cutu_t1", "cutu_t4"]
groups = {(p, s): [(p, c, s) for c in CF[(i + s) % 2:] + CF[:(i + s) % 2]] for i, p in enumerate(PROBS) for s in (1, 2, 3)}
gl = sorted(groups.items(), key=lambda kv: -sum(est(*t) for t in kv[1]))
load = [0.0] * NS; shards = [[] for _ in range(NS)]
for _, ts in gl:
    i = min(range(NS), key=lambda j: load[j]); shards[i] += [list(t) + [TL] for t in ts]; load[i] += sum(est(*t) for t in ts)
(D / "shards_c12.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print(f"shards_c12.json: {len(PROBS)} 問 × 種 3 × 2 設定 = {sum(len(s) for s in shards)} 回、{NS} 分担、見積り (時間) 各 {min(load)/3600:.2f}〜{max(load)/3600:.2f}")

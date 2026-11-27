"""第 2 段の分担表: python plan2.py → shards2.json (P = 58 問 × 種 1〜3 × cfg.py の 6 番以降の設定、60 秒。既定の行は第 1 段のものを使う)。
結果は python run.py K analysis/objprop_20261124/shards2.json analysis/objprop_20261124/res2_sK.jsonl。
1 スレッドの設定は分担 0〜2 (同時に 3 本回す)、4 スレッドの設定は分担 3 (1 本だけで回す)。同じ (問題, 種) の設定は同じ分担で続けて解き、順は組ごとに回す。
見積り (1 回、秒): analysis/final_20261118 の P の def (4 スレッド)・def_t1 の時間 (解けなければ 60) + 待ち 8 秒。"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D)); sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from cfg import CFGS, P1  # noqa: E402
NEW = {f"{P1[k][0]}_{th}" for k in P1 if k >= 6 for th, on in (("t1", P1[k][2]), ("t4", P1[k][3])) if on}
CFGS = {c: e for c, e in CFGS.items() if c in NEW}
from probs import PROBS  # noqa: E402
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
shards = [[] for _ in range(4)]; load = [0.0] * 4
for th, idx in (("t1", [0, 1, 2]), ("t4", [3])):
    CF = [c for c in CFGS if c.endswith("_t1") == (th == "t1")]
    groups = {(p, s): [(p, c, s) for c in CF[(i + s) % len(CF):] + CF[:(i + s) % len(CF)]] for i, p in enumerate(PROBS) for s in (1, 2, 3)}
    for _, ts in sorted(groups.items(), key=lambda kv: -sum(est(*t) for t in kv[1])):
        i = min(idx, key=lambda j: load[j]); shards[i] += [list(t) + [TL] for t in ts]; load[i] += sum(est(*t) for t in ts)
(D / "shards2.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print(f"shards2.json: {sum(len(s) for s in shards)} 回、見積り (時間) {[round(x / 3600, 2) for x in load]}")

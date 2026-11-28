"""分担表 (POLISH_BG の P の計測、analysis/parsb_20261126/plan.py を元にした): python plan.py → shards.json。
P = MIPLIB の 58 問 × 種 1〜3 × cfg.py の全設定、60 秒。分担 0 = 2 スレッド (def_t2・i_t2)、分担 1 = 4 スレッド (def_t4・i_t4)。
同じ (問題, 種) の 2 つの設定は同じ分担で続けて解き、順は組ごとに入れ替える。
見積り (1 回、秒): analysis/final_20261118 の P の def (4 スレッド) の同じ (問題, 種) の時間 (解けなければ 60)、2 スレッドも同じとみる、+ 待ち 8 秒。"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent
A = D.parents[1]
sys.path.insert(0, str(D)); sys.path.insert(0, str(A / "cbcimpl_20261029"))
from cfg import CFGS  # noqa: E402
from probs import PROBS  # noqa: E402
TL = 60
def solved(r): return str(r["result"].get("status", "")).lower() in ("optimal", "infeasible")
T = {}
for f in glob.glob(str(A / "final_20261118" / "res_P_s*.jsonl")):
    for l in open(f):
        if l.strip():
            r = json.loads(l)
            if r["cfg"] == "def":
                T[(r["problem"], r["seed"])] = min(r["result"]["time"], TL) if solved(r) else TL
def est(p, c, s): return T.get((p, s), TL) + 8
shards, load = [], []
for th in ("t2", "t4"):
    CF = [c for c in CFGS if c.endswith(th)]
    sh = []
    for i, p in enumerate(PROBS):
        for s in (1, 2, 3):
            r = (i + s) % len(CF)
            sh += [[p, c, s, TL] for c in CF[r:] + CF[:r]]
    shards.append(sh); load.append(sum(est(p, c, s) for p, c, s, _ in sh))
(D / "shards.json").write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print(f"shards.json: {len(PROBS)} 問 × 種 3 × {len(CFGS)} 設定 = {sum(len(s) for s in shards)} 回、分担 2 (2・4 スレッド)、見積り (時間) {[round(x / 3600, 2) for x in load]}")

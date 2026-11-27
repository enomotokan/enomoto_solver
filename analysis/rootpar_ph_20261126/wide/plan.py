"""RINS_PAR (根の RINS を並列のサブ MIP にして待たずに木へ) の広い計測の分担表 (どれも 4 スレッド。objstepcut_20261122/wide/plan.py の写しで、設定の組を引数で選ぶ)。
python plan.py P 6 def,rp  → shards_P.json (MIPLIB の 58 問 × 種 1〜3、60 秒)。見積りは analysis/final_20261118 の P の def の時間 + 8 秒。
python plan.py M 20 def,rp → shards_M.json (Mittelmann 186 問 × 種 1、600 秒)。見積りは analysis/vsscip_20261120 の M の def の時間 (解けなかった回は上限) + 30 秒。
設定の順は問題・種でずらす。設定の中身は cfg.py"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parents[1] / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
SET, NS, CF = sys.argv[1], int(sys.argv[2]), sys.argv[3].split(",")
def solved(r): return str(r["result"].get("status", "")).lower() in ("optimal", "infeasible")
def times(pat, cap):
    T = {}
    for f in glob.glob(str(pat)):
        for l in open(f):
            if not l.strip(): continue
            r = json.loads(l)
            if r["cfg"] == "def" and "time" in r["result"]: T[(r["problem"], r["seed"])] = min(r["result"]["time"], cap) if solved(r) else cap
    return T
def rot(i): k = i % len(CF); return CF[k:] + CF[:k]
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -sum(est(*t) for t in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for _, ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += sum(est(*t) for t in ts)
    (D / out).write_text(json.dumps({"cfgs": CF, "shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜",
          max(round(x / 3600, 2) for x in load), "計", round(sum(load) / 3600, 1))
if SET.startswith("P"):  # P (w・b の回)、P2 (4 スレッドの d・eg・egj)、P1 (1 スレッドの def_t1・eg_t1・egj_t1)。分担表は shards_<SET>.json
    TP = times(D.parents[1] / "final_20261118" / "res_P_s*.jsonl", 60)
    balance({(p, s): [(p, c, s) for c in rot(i + s)] for i, p in enumerate(PROBS) for s in (1, 2, 3)},
            lambda p, c, s: TP.get((p, s), TP.get((p, 1), 60)) + 8, NS, f"shards_{SET}.json", 60)
elif SET == "M":
    TM = times(D.parents[1] / "vsscip_20261120" / "res_M_s*.jsonl", 600)
    MP = sorted({x for part in json.loads((D.parents[1] / "cbcimpl_20261029/mitt_parts.json").read_text()) for x in part})
    balance({(p, 1): [(p, c, 1) for c in rot(i)] for i, p in enumerate(MP)}, lambda p, c, s: TM.get((p, s), 600) + 30, NS, "shards_M.json", 600)

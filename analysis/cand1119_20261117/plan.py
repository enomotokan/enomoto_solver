"""分担表: python plan.py NP NM → shards_P.json (58 問 × 種 1〜6 × {def, cand1119}、60 秒) と shards_M.json (重い問題 × 種 1〜3 × {def, cand1119}、600 秒)。
同じ (問題, 種) の 2 設定は同じ分担で続けて解き、順は種で入れ替える。見積りは cand1118 の回の時間 (P は cand1118_default の def、M は cand1118 の
cand1118。時間切れは制限時間、記録のない問題は制限時間) + 立ち上げ。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
# M: rootgap の M4 の 12 問 + これまでの M の重い問題 + 1 ラウンドの LP が重い大きめの Mittelmann の問題 (cand1118 か HiGHS が 600 秒で解いたもの)
M4 = ["mzzv11", "mzzv42z", "irp", "hypothyroid-k1", "tbfp-network", "neos-933966", "mcsched", "csched008", "neos-1456979",
      "neos-3216931-puriri", "traininstance6", "satellites2-60-fs"]
MOLD = ["neos-3627168-kasai", "rococoC10-001000", "supportcase33", "unitcal_7", "net12"]
MBIG = ["neos-4722843-widden", "CMS750_4", "neos-4738912-atrato", "fastxgemm-n2r6s0t2", "istanbul-no-cutoff"]
MPROBS = M4 + MOLD + MBIG
def times(pat, cfg, cap):
    T = defaultdict(list)
    for f in glob.glob(str(pat)):
        for l in open(f):
            r = json.loads(l)
            if r["cfg"] != cfg: continue
            st = str(r["result"].get("status") or "").lower()
            T[r["problem"]].append(min(r["result"].get("time") or cap, cap) if st in ("optimal", "infeasible") else cap)
    return {k: sum(v) / len(v) for k, v in T.items()}
def balance(groups, est, n, out):
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for key, runs in sorted(groups, key=lambda kv: -est(kv[0][0]) * len(kv[1])):
        i = min(range(n), key=lambda j: load[j]); shards[i] += runs; load[i] += est(key[0]) * len(runs)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, n, "分担、", sum(len(s) for s in shards), "回、見積り (時間)", [round(x / 3600, 2) for x in load])
CF = ["def", "cand1119"]
def order(p, s, tl):
    return [[p, c, s, tl] for c in (CF if s % 2 else CF[::-1])]
if len(sys.argv) > 2 and sys.argv[2] == "A":
    # 切り分け: python plan.py NA A → shards_A.json (4 問 × 種 1〜3 × 5 設定、600 秒。同じ (問題, 種) の 5 設定は同じ分担で、順は種で回す)
    CA = ["def", "cand1119", "no_sig", "no_twins", "no_aff"]
    TA = defaultdict(list)
    for f in glob.glob(str(D / "res_M_s*.jsonl")):
        for l in open(f):
            r = json.loads(l); st = str(r["result"].get("status") or "").lower()
            TA[r["problem"]].append(min(r["result"].get("time") or 600, 600) if st in ("optimal", "infeasible") else 600)
    est = lambda p: sum(TA[p]) / len(TA[p]) + 40 if TA[p] else 640
    groups = [((p, s), [[p, c, s, 600] for c in CA[s % 5:] + CA[:s % 5]]) for p in ["csched008", "neos-933966", "CMS750_4", "supportcase33"] for s in (1, 2, 3)]
    balance(groups, lambda p: est(p), int(sys.argv[1]), "shards_A.json")
    sys.exit()
NP, NM = int(sys.argv[1]), int(sys.argv[2])
TP = times(D.parent / "cand1118_default_20261116/res_P_s*.jsonl", "def", 60)
balance([((p, s), order(p, s, 60)) for p in PROBS for s in range(1, 7)], lambda p: TP.get(p, 60.0) + 8, NP, "shards_P.json")
TM = times(D.parent / "cand1118_20261116/res_M_s*.jsonl", "cand1118", 600)
balance([((p, s), order(p, s, 600)) for p in MPROBS for s in (1, 2, 3)], lambda p: TM.get(p, 600.0) + 40, NM, "shards_M.json")

"""分担表: python plan.py NP NM → shards_P.json (58 問 × 種 1〜3 × {def, pc, pcd}、60 秒) と shards_M.json (重い問題 × 種 1・2 × {def, pc, pcd}、600 秒)。
同じ (問題, 種) の 3 設定は同じ分担で続けて解き、順は種で回す。見積りは cand1119 の回の def の時間 (時間切れは制限時間、記録のない問題は制限時間)
× 3 設定 + 立ち上げ。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
sys.path.insert(0, str(D.parent / "cand1119_20261117"))
from probs import PROBS  # noqa: E402
# M: cand1119 の M の 22 問 (rootgap の M4・これまでの重い問題・1 ラウンドの LP が重い問題。大きい M の行を持つ数値の難しい問題
# (tbfp-network・neos-3216931-puriri・neos-1456979・supportcase33・unitcal_7 など) を含む) + sat 系 4 問
M4 = ["mzzv11", "mzzv42z", "irp", "hypothyroid-k1", "tbfp-network", "neos-933966", "mcsched", "csched008", "neos-1456979",
      "neos-3216931-puriri", "traininstance6", "satellites2-60-fs"]
MOLD = ["neos-3627168-kasai", "rococoC10-001000", "supportcase33", "unitcal_7", "net12"]
MBIG = ["neos-4722843-widden", "CMS750_4", "neos-4738912-atrato", "fastxgemm-n2r6s0t2", "istanbul-no-cutoff"]
MSAT = ["satellites2-40", "satellites2-25", "satellites3-25", "satellites4-25"]
MPROBS = M4 + MOLD + MBIG + MSAT
CF = ["def", "pc", "pcd"]
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
def order(p, s, tl):
    k = s % 3
    return [[p, c, s, tl] for c in CF[k:] + CF[:k]]
NP, NM = int(sys.argv[1]), int(sys.argv[2])
TP = times(D.parent / "cand1119_20261117/res_P_s*.jsonl", "def", 60)
balance([((p, s), order(p, s, 60)) for p in PROBS for s in (1, 2, 3)], lambda p: TP.get(p, 60.0) + 8, NP, "shards_P.json")
TM = times(D.parent / "cand1119_20261117/res_M_s*.jsonl", "def", 600)
balance([((p, s), order(p, s, 600)) for p in MPROBS for s in (1, 2)], lambda p: TM.get(p, 600.0) + 40, NM, "shards_M.json")

"""分担表: python plan.py NM NP → shards_M.json・shards_P.json (c18・c18_clqroot・c18_clqmarg)。
python plan.py NM NP S → shards_SM.json・shards_SP.json (シグモイドの比べ: c18・c18_clqroot・c18_sig・c18_siga。README の 2 節)
P: 58 問 × 種 1〜4 × 設定、60 秒。M: mzzv11・mzzv42z・irp・hypothyroid-k1・tbfp-network × 種 1・2 × 設定、600 秒。
同じ (問題, 種) の 3 設定は同じ分担で続けて解き、順は種で回す。見積りは cand1118 の回の cand1118 の時間 (+ 立ち上げ)。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
C18 = D.parents[1] / "cand1118_20261116"
sys.path.insert(0, str(D.parents[1] / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
from cfg import CFGS  # noqa: E402
NM, NP = int(sys.argv[1]), int(sys.argv[2])
PRE = sys.argv[3] if len(sys.argv) > 3 else ""
CF = ["c18", "c18_clqroot", "c18_sig", "c18_siga"] if PRE == "S" else ["c18", "c18_clqroot", "c18_clqmarg"]
MPROBS = ["mzzv11", "mzzv42z", "irp", "hypothyroid-k1", "tbfp-network"]
def times(pat, cfg, cap):
    T = defaultdict(list)
    for f in glob.glob(str(pat)):
        for l in open(f):
            r = json.loads(l)
            if r["cfg"] != cfg: continue
            T[r["problem"]].append(min(r["result"].get("time") or cap, cap))
    return {k: sum(v) / len(v) for k, v in T.items()}
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -sum(est(t[0]) for t in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for (p, s), ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += sum(est(t[0]) for t in ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", [round(x / 3600, 2) for x in load])
def order(p, s):
    k = s % len(CF)
    return [(p, c, s) for c in CF[k:] + CF[:k]]
TM = times(C18 / "res_M_s*.jsonl", "cand1118", 600)
balance({(p, s): order(p, s) for p in MPROBS for s in (1, 2)}, lambda p: TM.get(p, 600.0) + 30, NM, f"shards_{PRE}M.json", 600)
TP = times(C18 / "res_P_s*.jsonl", "cand1118", 60)
balance({(p, s): order(p, s) for p in PROBS for s in (1, 2, 3, 4)}, lambda p: TP.get(p, 48.0) + 8, NP, f"shards_{PRE}P.json", 60)

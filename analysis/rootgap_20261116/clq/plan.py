"""分担表: python plan.py NM NP → shards_M.json・shards_P.json (c18・c18_clqroot・c18_clqmarg)。
python plan.py NM NP 5 → shards_M5.json・shards_P5.json (双子の併合: c18・c18_parint・c18_twins、M5 は 8 問 × 種 1〜4。README の 6 節)。
python plan.py NM 0 4 → shards_M4.json (M の 5 問 + clique の効きそうな 600 秒級の 7 問 × 種 1〜4、c18 と c18_sig。README の 5 節)。
python plan.py NM NP 3 → shards_M3.json・shards_P3.json (c18 と c18_sig を同じ分担で並べる確かめ。README の 4 節)。
python plan.py NM NP 2 → shards_M2.json・shards_P2.json (シグモイドの比べの c18_sig・c18_siga だけ。c18・c18_clqroot は P・M の結果を使う。README の 2 節)
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
SUF = sys.argv[3] if len(sys.argv) > 3 else ""
CF = {"2": ["c18_sig", "c18_siga"], "3": ["c18", "c18_sig"], "4": ["c18", "c18_sig"], "5": ["c18", "c18_parint", "c18_twins"]}.get(SUF, ["c18", "c18_clqroot", "c18_clqmarg"])
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
# M4 に足す 7 問 (集合詰め込み・分割の行が多いか衝突の多い問題で、cand1118 か HiGHS が 600 秒で解いたもの)
M4ADD = ["neos-933966", "mcsched", "csched008", "neos-1456979", "neos-3216931-puriri", "traininstance6", "satellites2-60-fs"]
if SUF == "5":
    M5 = ["irp", "mzzv11", "mzzv42z", "hypothyroid-k1", "neos-933966", "mcsched", "neos-1456979", "traininstance6"]
    balance({(p, s): order(p, s) for p in M5 for s in (1, 2, 3, 4)}, lambda p: TM.get(p, 600.0) + 40, NM, "shards_M5.json", 600)
    TP = times(C18 / "res_P_s*.jsonl", "cand1118", 60)
    balance({(p, s): order(p, s) for p in PROBS for s in (1, 2, 3, 4)}, lambda p: TP.get(p, 48.0) + 8, NP, "shards_P5.json", 60)
    sys.exit()
if SUF == "4":
    balance({(p, s): order(p, s) for p in MPROBS + M4ADD for s in (1, 2, 3, 4)}, lambda p: TM.get(p, 600.0) + 40, NM, "shards_M4.json", 600)
    sys.exit()
balance({(p, s): order(p, s) for p in MPROBS for s in (1, 2)}, lambda p: TM.get(p, 600.0) + 30, NM, f"shards_M{SUF}.json", 600)
TP = times(C18 / "res_P_s*.jsonl", "cand1118", 60)
balance({(p, s): order(p, s) for p in PROBS for s in (1, 2, 3, 4)}, lambda p: TP.get(p, 48.0) + 8, NP, f"shards_P{SUF}.json", 60)

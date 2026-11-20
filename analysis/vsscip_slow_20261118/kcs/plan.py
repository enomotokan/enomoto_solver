"""群 2 kcs の対の比べの分担表 (slow2_20261117/plan.py の写し): python plan.py NM NP [設定 [M に足す問題,...]] → shards_M<札>.json・shards_P<札>.json (札は kcs なら空、sw なら SW)
M: Mittelmann 48 問 (vshighs の M と同じ) × 種 1・2 × {def, kcs}、600 秒。P: MIPLIB の 58 問 × 種 1〜3 × {def, kcs}、60 秒。
同じ (問題, 種) の 2 組は同じ分担で続けて解き、順は種で入れ替える。見積り: vshighs_20261117 の def の時間 (+ 30 秒・8 秒)。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
VS = D.parents[1] / "vshighs_20261117"
sys.path.insert(0, str(D.parents[1] / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
NM, NP = int(sys.argv[1]), int(sys.argv[2])
ALT = sys.argv[3] if len(sys.argv) > 3 else "kcs"  # 既定と対にする設定
SUF = "" if ALT == "kcs" else ALT.upper()
XM = sys.argv[4].split(",") if len(sys.argv) > 4 else []  # M に足す問題 (例: neos-2746589-doon、見積りは 600 秒)
def times(pat, cfg, cap):
    T = defaultdict(list)
    for f in glob.glob(str(pat)):
        for l in open(f):
            r = json.loads(l)
            if r["cfg"] != cfg: continue
            t = r["result"].get("time") or cap
            T[r["problem"]].append(min(t, cap))
    return {k: sum(v) / len(v) for k, v in T.items()}
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -sum(est(t[0], t[1]) for t in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for (p, s), ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += sum(est(t[0], t[1]) for t in ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", min(round(x / 3600, 2) for x in load), "〜", max(round(x / 3600, 2) for x in load))
def order(p, s, c):
    k = s % 2
    return [(p, x, s) for x in c[k:] + c[:k]]
TM = times(VS / "res_M_s*.jsonl", "def", 600)
MPROBS = sorted(TM) + [x for x in XM if x not in TM]
def estM(p, c): return TM.get(p, 600.0) + 30
balance({(p, s): order(p, s, ["def", ALT]) for p in MPROBS for s in (1, 2)}, estM, NM, f"shards_M{SUF}.json", 600)
TP = times(VS / "res_P_s*.jsonl", "def", 60)
def estP(p, c): return TP.get(p, 48.0) + 8
balance({(p, s): order(p, s, ["def", ALT]) for p in PROBS for s in (1, 2, 3)}, estP, NP, f"shards_P{SUF}.json", 60)

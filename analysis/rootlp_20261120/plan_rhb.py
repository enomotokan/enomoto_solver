"""RHBUDGET の分担表: python plan_rhb.py NM NP → shards_M.json・shards_P.json (scipslow_20261119/plan.py の写し)
M: pooled_probs.json の M (根で Pump・ダイビングが回る 32 問、rhscan の 1 スレッドの根だけの走査) × 種 1・2 × {def, rhb}、600 秒・4 スレッド。
P: MIPLIB の 58 問 × 種 1〜3 × {def, rhb}、60 秒・4 スレッド。見積り: P は vshighs_20261117、M は vsscip_20261117 の def の時間 (+ 30 秒・8 秒)。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
A = D.parents[0]
sys.path.insert(0, str(A / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
ALT = "rhb"
def times(pat, cfg, cap):
    T = defaultdict(list)
    for f in glob.glob(str(pat)):
        for l in open(f):
            r = json.loads(l)
            if r["cfg"] != cfg: continue
            T[r["problem"]].append(min(r["result"].get("time") or cap, cap))
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
NM, NP = int(sys.argv[1]), int(sys.argv[2])
MP = json.load(open(D / "pooled_probs.json"))["M"]
TM = times(A / "vsscip_20261117/res_M_s*.jsonl", "def", 600)
balance({(p, s): order(p, s, ["def", ALT]) for p in MP for s in (1, 2)}, lambda p, c: TM.get(p, 600.0) + 30, NM, "shards_M.json", 600)
TP = times(A / "vshighs_20261117/res_P_s*.jsonl", "def", 60)
balance({(p, s): order(p, s, ["def", ALT]) for p in PROBS for s in (1, 2, 3)}, lambda p, c: TP.get(p, 48.0) + 8, NP, "shards_P.json", 60)

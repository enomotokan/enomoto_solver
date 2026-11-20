"""XO (ENOMOTO_XO_BUMP_LU_RATIO) の分担表: python plan_xo.py → shards_PXO.json・shards_MXO.json
PXO: MIPLIB の 58 問 × 種 1〜3 × {def, xo, def_t1, xo_t1}、60 秒、8 分担。
MXO: 走査 (xoscan.jsonl) で押し出しの再分解 + 双対の押し出しが 0.1 秒以上の M の 7 問 × 種 1・2 × {def, xo, def_t1, xo_t1}、600 秒、5 分担。
同じ (問題, 種) の組は同じ分担で続けて解き、順は種で入れ替える。見積り: P は vshighs_20261117 の def の時間 + 8 秒 (1 スレッドは 2 倍、上限 60)、M は上限 + 30 秒。"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
CF = ["def", "xo", "def_t1", "xo_t1"]
def balance(groups, est, n, out, tl):
    gl = sorted(groups.items(), key=lambda kv: -sum(est(*t[:2]) for t in kv[1]))
    load = [0.0] * n; shards = [[] for _ in range(n)]
    for _, ts in gl:
        i = min(range(n), key=lambda j: load[j]); shards[i] += [list(t) + [tl] for t in ts]; load[i] += sum(est(*t[:2]) for t in ts)
    (D / out).write_text(json.dumps({"shards": shards, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
    print(out, len(groups), "組", sum(len(s) for s in shards), "回", "見積り (時間):", [round(x / 3600, 2) for x in load])
def order(p, s):
    k = s % len(CF)
    return [(p, x, s) for x in CF[k:] + CF[:k]]
T = defaultdict(list)
for f in glob.glob(str(D.parent / "vshighs_20261117" / "res_P_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "def": T[r["problem"]].append(min(r["result"].get("time") or 60, 60))
TP = {k: sum(v) / len(v) for k, v in T.items()}
def estP(p, c):
    t = TP.get(p, 48.0)
    return (min(2 * t, 60) if c.endswith("_t1") else t) + 8
balance({(p, s): order(p, s) for p in PROBS for s in (1, 2, 3)}, estP, 8, "shards_PXO.json", 60)
MP = ["cod105", "neos-1354092", "chromaticindex512-7", "radiationm40-10-02", "neos-4300652-rahue", "mzzv11", "dano3_5"]
balance({(p, s): order(p, s) for p in MP for s in (1, 2)}, lambda p, c: 630.0, 5, "shards_MXO.json", 600)

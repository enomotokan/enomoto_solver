"""LPPROBE_TICKS の分担表: python plan_tk.py → shards_PTK.json・shards_MTK.json
PTK: MIPLIB の 58 問 × 種 1〜3 × {def, tk, tkw}、60 秒・4 スレッド、3 分担。
MTK: 走査 (tk_mprobs.json = 試作で LP による固定の固定が減るか時間が 0.5 秒以上変わる M の 17 問) + cod105 × 種 1・2 × {def, tk, tkw}、600 秒・4 スレッド、13 分担。
同じ (問題, 種) の組は同じ分担で続けて解き、順は種で入れ替える。見積り: P は vshighs_20261117 の def の時間 + 8 秒、M は上限 + 30 秒 (子の実行環境の 2 時間に収まる大きさ)。"""

import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
CF = ["def", "tk", "tkw"]
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
    return TP.get(p, 48.0) + 8
balance({(p, s): order(p, s) for p in PROBS for s in (1, 2, 3)}, estP, 3, "shards_PTK.json", 60)
MP = json.loads((D / "tk_mprobs.json").read_text()) + ["cod105"]
balance({(p, s): order(p, s) for p in MP for s in (1, 2)}, lambda p, c: 630.0, 13, "shards_MTK.json", 600)

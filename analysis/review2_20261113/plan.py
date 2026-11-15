"""REVIEW2 の分担表 shards_M.json を作る: 17 問 × 種 1・2 × 11 設定 (base + 10 項目) × 600 秒。
同じ (問題, 種) の 11 設定は同じ分担で続けて回す (設定の順は (問題, 種) ごとに回して並べ、機械の時間の偏りを散らす)。
見積りは cand1111・cand1113 の Mittelmann の回の時間 (+ 10 秒) から。"""
import json
from pathlib import Path
from cfg import CFGS
D = Path(__file__).resolve().parent
TL = 600
EST = {  # 1 回の見積り (秒)
    "supportcase33": 600, "fiball": 600, "unitcal_7": 600, "neos-787933": 560, "tr12-30": 520, "CMS750_4": 420, "hypothyroid-k1": 400,
    "mzzv42z": 230, "pg5_34": 220, "neos-4722843-widden": 80, "h80x6320d": 80, "neos859080": 50, "physiciansched6-2": 40,
    "enlight_hard": 25, "cbs-cta": 20, "neos-1122047": 15, "neos-960392": 10,
}
NSH = 8
cfgs = list(CFGS)
groups = sorted(((p, s) for p in EST for s in (1, 2)), key=lambda g: -EST[g[0]])
load = [0.0] * NSH; shards = [[] for _ in range(NSH)]
for gi, (p, s) in enumerate(groups):
    k = min(range(NSH), key=lambda i: load[i])
    rot = gi % len(cfgs)
    for c in cfgs[rot:] + cfgs[:rot]:
        shards[k].append([p, c, s, TL])
    load[k] += len(cfgs) * (EST[p] + 10)
(D / "shards_M.json").write_text(json.dumps({"est_hours": [round(x / 3600, 2) for x in load], "shards": shards}, ensure_ascii=False, indent=0))
print([round(x / 3600, 2) for x in load], sum(len(s) for s in shards))

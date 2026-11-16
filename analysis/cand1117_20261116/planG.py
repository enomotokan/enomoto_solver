"""DFSBB=2 (HiGHS 型) の部品の切り分け G: 損をした supportcase40 と、得をした noswot・timtab1・roll3000 (P、60 秒)・rocI-4-11・mcsched (M、600 秒)
× 種 1〜4 × 8 設定 (cand1117・s_nodfsbb と cfg.py の HGS の 6 通り: g_p10・g_p30 = 潜りの上限 10・30、g_k1 = いつも下界最小 (選び方を下界のみ
にした形と同じ)、g_pq = 潜りを子の下界で状況で打ち切る、g_sq = 兄弟へ戻る割合を既定に、g_sib = 兄弟のノードでも分離)、4 スレッド (192 回)。
600 秒の問題は (問題, 種) の 8 設定を 4 設定ずつ 2 つに分け、60 秒の問題は 8 設定まとめて、見積り 45 分以内に詰める。
見積りは cand1117 の M・S・P の時間 (+ 立ち上げ)。python planG.py N → shards_G.json。回し方 python run.py G K、集計 python summ.py G。
**ENOMOTO_MIP_HGS_OPT を足した版 (このコミット以降) でビルドして回す。**"""
import json, glob, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
CF = ["cand1117", "s_nodfsbb", "g_p10", "g_p30", "g_k1", "g_pq", "g_sq", "g_sib"]
TL = {"supportcase40": 600, "rocI-4-11": 600, "mcsched": 600, "noswot": 60, "timtab1": 60, "roll3000": 60}
T = defaultdict(list)
for f in glob.glob(str(D / "res_[MSPX]_s*.jsonl")):
    for l in open(f):
        r = json.loads(l)
        if r["problem"] in TL and r["cfg"] == "cand1117":
            T[r["problem"]].append(min(r["result"].get("time") or TL[r["problem"]], TL[r["problem"]]))
est = {p: (sum(v) / len(v) if v else TL[p]) + (30 if TL[p] == 600 else 8) for p, v in T.items()}
groups = []
for p, tl in TL.items():
    for s in (1, 2, 3, 4):
        k = s % len(CF); c = CF[k:] + CF[:k]
        parts = [c[:4], c[4:]] if tl == 600 else [c]
        for part in parts:
            groups.append(([[p, x, s, tl] for x in part], len(part) * est[p], len(part) * (tl + (30 if tl == 600 else 8))))
N = int(sys.argv[1])
groups.sort(key=lambda g: -g[1])
load = [0.0] * N; worst = [0.0] * N; shards = [[] for _ in range(N)]
for runs, e, w in groups:
    i = min(range(N), key=lambda j: load[j]); shards[i] += runs; load[i] += e; worst[i] += w
(D / "shards_G.json").write_text(json.dumps({"shards": shards, "cfgs": CF, "est_hours": [round(x / 3600, 2) for x in load]}, indent=0))
print("shards_G.json:", N, "分担、", sum(len(s) for s in shards), "回、見積り (時間)", min(load) / 3600, "〜", max(load) / 3600, "、最悪", max(worst) / 3600, "見積り", {p: round(v) for p, v in est.items()})

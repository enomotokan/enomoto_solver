"""supportcase40 の切り分け S: cand1117 は 4 種とも cand1116 より遅い (1.35・1.58・2.10・1.32、3 種が時間切れ)。最適解 24256.31 は早く持つが
下界が 23994 付近で止まる (証明が遅い) ので、DFSBB=2 の下界最小を取る頻度の低さ (k10) を疑う。python planS.py → shards_S.json
設定: cand1116・cand1117・s_nodfsbb (cand1117 から DFSBB を外す)・s_k5・s_k3 (ENOMOTO_T_MIP_HGS_BB_EVERY=5・3)・c16+dfsbb (cand1116 + DFSBB=2)
× 種 1〜4、600 秒・4 スレッド (24 回)。1 分担 = 1 種の 3 設定 (最大約 32 分)、8 分担。回し方 python run.py S K (K = 0〜7)、集計 python summ.py S。"""
import json
from pathlib import Path
D = Path(__file__).resolve().parent
CF = ["cand1116", "cand1117", "s_nodfsbb", "s_k5", "s_k3", "c16+dfsbb"]
shards = []
for s in (1, 2, 3, 4):
    k = s % len(CF); c = CF[k:] + CF[:k]
    shards += [[["supportcase40", x, s, 600] for x in c[:3]], [["supportcase40", x, s, 600] for x in c[3:]]]
(D / "shards_S.json").write_text(json.dumps({"shards": shards, "cfgs": CF}, indent=0))
print("shards_S.json:", len(shards), "分担 ×", [len(x) for x in shards], "回")

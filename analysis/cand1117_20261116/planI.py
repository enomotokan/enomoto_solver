"""切り分け I: P で cand1117 が cand1114 より全種で遅かった 10teams (3.54)・neos-827175 (1.59) の原因を分ける。python planI.py → shards_I.json
設定 (cfg.py の ISO): cand1114・cand1116・cand1117、cand1117 から 1 つずつ外した 10 通り (m_*: cand1114 → cand1116 の 7 鍵と、TICKS・DETHASH・DFSBB)、
cand1116 に 1 つずつ足した 3 通り (c16+*)。2 問 × 種 1〜8 × 16 設定、60 秒・4 スレッド (256 回)。1 分担 = 種 2 つ (64 回、見積り約 30 分)、4 分担。
回し方 python run.py I K (K = 0〜3)、集計 python summI.py。neos-827175 は 3 設定とも根で解けている (nodes 0) ので木の進め方 (DFSBB) ではない見込み。"""
import json
from pathlib import Path
from cfg import ISO
D = Path(__file__).resolve().parent
CF = ["cand1114", "cand1116", "cand1117"] + list(ISO)
shards = []
for a in (1, 3, 5, 7):
    sh = []
    for s in (a, a + 1):
        for p in ("10teams", "neos-827175"):
            k = (s * 5) % len(CF)
            sh += [[p, c, s, 60] for c in CF[k:] + CF[:k]]
    shards.append(sh)
(D / "shards_I.json").write_text(json.dumps({"shards": shards, "cfgs": CF}, indent=0))
print("shards_I.json:", len(shards), "分担 ×", len(shards[0]), "回、設定", len(CF), CF)

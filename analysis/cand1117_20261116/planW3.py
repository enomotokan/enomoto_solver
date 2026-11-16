"""誤答の再現 W3: 対称性を切って (cfg.py の NOSYM) デバッグ解の検査を正確にした再現。python planW3.py → shards_W3.json
cand1116・cand1117 × unitcal_7 × 種 1〜16、600 秒・4 スレッド (32 回)。1 分担 4 回 (最大約 42 分)、8 分担。
**db4c6b476 以降 (できれば最新) でビルドして回す** (サブ MIP の誤検知を直した版。この後の版はノードの伝播・局所の被約費用の固定で
最適解を落としたとき、段の名前と、外した境界の出どころ・最適解を破る LP の行を出す)。
回し方 python run.py W3 K (K = 0〜7)、ログは logs/W3_設定_s種.log.gz、集計 python summW.py W3 > summ_W3.txt。"""
import json
from pathlib import Path
D = Path(__file__).resolve().parent
CF = ["w16_nosym", "w17_nosym"]
runs = [["unitcal_7", c, s, 600] for s in range(1, 17) for c in CF]
N = 8
shards = [runs[i::N] for i in range(N)]
(D / "shards_W3.json").write_text(json.dumps({"shards": shards, "cfgs": CF}, indent=0))
print("shards_W3.json:", N, "分担、回数", [len(s) for s in shards])

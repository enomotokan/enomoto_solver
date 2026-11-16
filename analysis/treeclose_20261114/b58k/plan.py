"""b58k: ラウンド R の提案 (proposals_rR.json) の分担表 shards_boR.json を作る: python plan.py R。1 設定 = 1 分担 (評価の 8 問 × 種 1〜2・600 秒、16 回・約 2.8 時間)。
回すのは python run_shard.py K boR (K = 設定の番号)、結果は bo_s<R>_<K>.jsonl ではなく boR_sK.jsonl (propose.py は bo*_s*.jsonl を読む)。"""
import json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
PROBS = ["mcsched", "supportcase40", "CMS750_4", "neos-3627168-kasai", "rococoC10-001000", "unitcal_7", "traininstance6", "tbfp-network"]
R = int(sys.argv[1])
props = json.loads((D / f"proposals_r{R}.json").read_text())["cfgs"]
shards = [[[p, c, s] for s in (1, 2) for p in PROBS] for c in props]
(D / f"shards_bo{R}.json").write_text(json.dumps({"tl": 600, "shards": shards, "est_hours": [round(len(x) * 620 / 3600, 2) for x in shards]}, indent=0))
print(len(shards), "分担、各", len(shards[0]), "回")

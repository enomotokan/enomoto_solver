#!/bin/sh
# 2 版目 (f・g もラプラスにならす、82071c738 以降の .so): L (狙いの 4 問、600 秒) → P (58 問、60 秒)。途中から再開できる。
cd "$(dirname "$0")"
python run.py 0 shards_L.json res_L2_s0.jsonl
python run.py 0 shards_P.json res_P2_s0.jsonl
echo 全部済み

#!/bin/sh
# 3 版目 (要る側の平均・事前を 0 寄り・潜りの途中でも呼ぶ): L (狙いの 4 問、600 秒) → P (58 問、60 秒)。途中から再開できる。
cd "$(dirname "$0")"
python run.py 0 shards_L.json res_L3_s0.jsonl
python run.py 0 shards_P.json res_P3_s0.jsonl
echo 全部済み

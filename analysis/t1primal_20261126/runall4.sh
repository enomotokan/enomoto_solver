#!/bin/sh
# 4 版目 (3 版目 + 再スタートの後も入れる、7d43fcdac 以降): L (狙いの 4 問、600 秒) → P (58 問、60 秒)。途中から再開できる。
cd "$(dirname "$0")"
python run.py 0 shards_L.json res_L4_s0.jsonl
python run.py 0 shards_P.json res_P4_s0.jsonl
echo 全部済み

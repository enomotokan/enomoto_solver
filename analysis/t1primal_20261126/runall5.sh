#!/bin/sh
# 5 版目 (取り分を h × (f+g)/2 に、再スタートで回数を引き継ぎ、暫定解がなければ打ち切り値で f・g): L (狙いの 4 問、600 秒) → P (58 問、60 秒)。途中から再開できる。
cd "$(dirname "$0")"
python run.py 0 shards_L.json res_L5_s0.jsonl
python run.py 0 shards_P.json res_P5_s0.jsonl
echo 全部済み

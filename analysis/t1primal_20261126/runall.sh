#!/bin/sh
# L (狙いの 4 問、600 秒) → P (58 問、60 秒) を順に回す。途中から再開できる (run.py が済んだ行を飛ばす)。
cd "$(dirname "$0")"
python run.py 0 shards_L.json res_L_s0.jsonl
python run.py 0 shards_P.json res_P_s0.jsonl
echo 全部済み

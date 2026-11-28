#!/bin/sh
# 5 版目を新しい既定 (d5956146f 以降) で測り直す (README 10 節)。分担 K を回す: sh runall5n.sh P K (K = 0〜2) か sh runall5n.sh L K (K = 0〜1)。
# 結果は res_P5n_sK.jsonl・res_L5n_sK.jsonl。途中から再開できる。
cd "$(dirname "$0")"
python run.py "$2" "shards_$15n.json" "res_$15n_s$2.jsonl"
echo 全部済み

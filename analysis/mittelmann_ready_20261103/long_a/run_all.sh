#!/bin/bash
# 組 A の 7 問を ENOMOTO・HiGHS の順に 1 つずつ (同時に 1 ソルバー) 600 秒・4 スレッドで解く
cd "$(dirname "$0")"
for p in h80x6320d neos-2978193-inde physiciansched6-2 hypothyroid-k1 timtab1 neos859080 drayage-25-23; do
  for k in ours highs; do
    python run_long.py $k $p --time-limit ${TL:-600} --threads 4 --out long.jsonl --logdir logs
  done
done

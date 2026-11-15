#!/bin/bash
# 計測の待ち行列 (担当 A): queue.txt の先頭の行を 1 つずつ取り出して実行し、done.txt に足す。ソルバーは同時に 1 つ。
# queue.txt が空なら 30 秒待つ。STOP ができたら終わる。行の形: python run1.py ours 問題 タグ 種 秒 [KEY=VAL ...]
cd "$(dirname "$0")"
while [ ! -e STOP ]; do
  line=$(head -1 queue.txt 2>/dev/null)
  if [ -z "$line" ]; then sleep 30; continue; fi
  sed -i 1d queue.txt
  echo "$(date +%H:%M:%S) start $line" >> queue.log
  out=$(eval "$line" 2>&1 | tail -1)
  echo "$(date +%H:%M:%S) $out" >> queue.log
  echo "$line" >> done.txt
done

#!/bin/bash
# queue.txt の行 (名前 問題 秒 種 [KEY=VAL ...]) を上から順に 1 つずつ回す。済んだ行は queue_done.txt へ移す (途中で足してよい)
cd "$(dirname "$0")"
while true; do
  line=$(head -1 queue.txt 2>/dev/null)
  [ -z "$line" ] && break
  sed -i 1d queue.txt
  python3 run1.py $line
  echo "$line" >> queue_done.txt
done

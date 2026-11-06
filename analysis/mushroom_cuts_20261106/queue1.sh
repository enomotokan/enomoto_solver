#!/bin/bash
# キュー 1 (2026-11-06): mushroom-best を 4 スレッド・600 秒で def・mc を 2 回ずつ (m600.jsonl)、候補 26 問を mc で 60 秒 1 回 (screen60.jsonl。
# def は intvub_20261106/screen60.jsonl)、続けて 58 問を def・mc で 2 回 (res58.jsonl)。ソルバーは同時に 1 つ。
cd "$(dirname "$0")"
python run.py def mushroom-best 600 1 m600.jsonl
python run.py mc mushroom-best 600 1 m600.jsonl
python run.py mc mushroom-best 600 2 m600.jsonl
python run.py def mushroom-best 600 2 m600.jsonl
for p in $(cat ../intvub_20261106/cand.txt); do
  [ "$p" = mushroom-best ] && continue
  python run.py mc $p 60 1 screen60.jsonl
done
./run58.sh 1 def mc
./run58.sh 2 def mc

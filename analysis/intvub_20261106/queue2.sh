#!/bin/bash
# キュー 2 (2026-11-06): キュー 1 の後に、ふるいで差の出た問題を 60 秒で 2〜4 回 (rep60.jsonl)、続けて 58 問を def・iv1 で 2 回 (res58.jsonl)
cd "$(dirname "$0")"
while pgrep -f queue1.sh > /dev/null; do sleep 30; done
for r in 2 3; do
  for p in neos859080 neos-1582420 noswot piperout-08 piperout-27 qnet1; do
    if [ $((r % 2)) -eq 0 ]; then cs="def iv1 iv3"; else cs="iv3 iv1 def"; fi
    for c in $cs; do python run.py $c $p 60 $r rep60.jsonl; done
  done
done
./run58.sh 1 def iv1
./run58.sh 2 def iv1

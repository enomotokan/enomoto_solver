#!/bin/bash
# キュー 1 (2026-11-06): 候補の問題 (cand.txt) を 60 秒で def・iv1・iv3 を 1 回ずつ (screen60.jsonl)、mushroom-best を 600 秒で def・iv1 を 2 回ずつ (m600.jsonl)
cd "$(dirname "$0")"
i=0
while read p; do
  if [ $((i % 2)) -eq 0 ]; then cs="def iv1 iv3"; else cs="iv3 iv1 def"; fi
  for c in $cs; do python run.py $c $p 60 1 screen60.jsonl; done
  i=$((i+1))
done < cand.txt
for r in 1 2; do for c in def iv1; do python run.py $c mushroom-best 600 $r m600.jsonl; done; done

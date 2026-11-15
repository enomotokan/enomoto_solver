#!/bin/bash
# 58 問 (analysis/cbcimpl_20261029/probs.py) を 4 スレッド・60 秒で、引数の設定を回ごとに 1 回ずつ (res58.jsonl)。問題ごとに設定の順を入れ替える。
# 使い方: ./run58.sh 回 設定 [設定 ...]
cd "$(dirname "$0")"
rep=$1; shift
i=0
for p in $(python -c "import sys; sys.path.insert(0, '../cbcimpl_20261029'); from probs import PROBS; print(' '.join(PROBS))"); do
  if [ $(((i + rep) % 2)) -eq 0 ]; then cs="$*"; else cs=$(echo "$*" | tr ' ' '\n' | tac | tr '\n' ' '); fi
  for c in $cs; do
    if [ -f res58.jsonl ] && grep -q "\"problem\": \"$p\", \"cfg\": \"$c\", \"tl\": 60, \"rep\": $rep," res58.jsonl; then continue; fi
    python run.py $c $p 60 $rep res58.jsonl
  done
  i=$((i+1))
done

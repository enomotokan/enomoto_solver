#!/bin/bash
# 使い方: batch.sh <回の接尾辞> <設定名>... (設定は configs.sh で定義)。問題は PROBS (既定 4 問)
cd "$(dirname "$0")/.."
source inv_cutsel/configs.sh
r=$1; shift
PROBS=${PROBS:-"neos-911970 roll3000 mzzv42z neos-2978193-inde"}
for c in "$@"; do for p in $PROBS; do
  [ -f inv_cutsel/logs/${p}_${c}_${r}.log ] && grep -q RESULT inv_cutsel/logs/${p}_${c}_${r}.log && continue
  python inv_cutsel/run.py $p ${c}_${r} ${T:-60} ${CFG[$c]} >/dev/null
  python inv_cutsel/summ.py inv_cutsel/logs/${p}_${c}_${r}.log
done; done

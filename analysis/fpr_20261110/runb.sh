#!/bin/bash
# (b): 58 問、既定と FPR を 4 スレッド・60 秒・種 1・2 (問題ごとに既定と FPR を続けて回す)
cd "$(dirname "$0")"
P=$(python3 -c "import sys; sys.path.insert(0,'../cbcimpl_20261029'); import probs; print(' '.join(probs.PROBS))")
for s in 1 2; do for p in $P; do for c in def fpr; do python run1.py b58.jsonl $c $s 60 $p; done; done; done

#!/bin/bash
# (b2): 58 問、inc_only_fj を直した FPR (fpr2) を 4 スレッド・60 秒・種 1・2。既定は (b) の b58.jsonl の def を使う
cd "$(dirname "$0")"
P=$(python3 -c "import sys; sys.path.insert(0,'../cbcimpl_20261029'); import probs; print(' '.join(probs.PROBS))")
for s in 1 2; do for p in $P; do python run1.py b58.jsonl fpr2 $s 60 $p; done; done

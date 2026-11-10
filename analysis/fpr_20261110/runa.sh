#!/bin/bash
# (a): 最初の解が遅い問題で既定と FPR を 4 スレッド・120 秒・種 1・2
P="neos-873061 neos-4722843-widden tbfp-network neos-1456979 cod105 satellites2-60-fs csched008 mzzv11 mzzv42z"
for s in 1 2; do for p in $P; do for c in def fpr; do python run1.py a120.jsonl $c $s 120 $p; done; done; done

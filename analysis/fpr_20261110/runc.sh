#!/bin/bash
# (c): 47 問の一部 (a で FPR の別スレッドが解を出したもの)、既定と FPR を 4 スレッド・600 秒・種 1・2
cd "$(dirname "$0")"
P="neos-1456979 mzzv42z satellites2-60-fs neos-4722843-widden"
for s in 1 2; do for p in $P; do for c in def fpr; do python run1.py c600.jsonl $c $s 600 $p; done; done; done

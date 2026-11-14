#!/bin/sh
# 本計測: 1 スレッド (4 本同時) で基準と spf、続けて 4 スレッドで基準と spf (58 問 × 6 回ずつ)
cd "$(dirname "$0")"
for i in 0 1 2 3; do (python run.py 1 $i 4 60 base; python run.py 1 $i 4 60 spf) > /dev/null 2>&1 & done
wait
python run.py 4 0 1 60 base > /dev/null 2>&1
python run.py 4 0 1 60 spf > /dev/null 2>&1

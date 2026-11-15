#!/bin/bash
# 止め方に敏感な問題 (60 秒 x 3 回、既定と試作 c を交互に)
cd "$(dirname "$0")"
for rep in 1 2 3; do
  for p in neos-960392 physiciansched6-2 neos5; do
    for cfg in def c; do python run.py $cfg 60 $rep $p; done
  done
done

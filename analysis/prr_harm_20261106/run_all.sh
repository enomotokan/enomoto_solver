#!/bin/sh
# 58 問 2 回、その後に 4 問の 600 秒を 2 回 (設定の順は回ごとに逆)
cd "$(dirname "$0")/../.."
D=analysis/prr_harm_20261106
python3 $D/bench58.py 50
python3 $D/bench58.py 51
for r in 1 2; do
  for p in momentum1 drayage-25-23 timtab1 traininstance6; do
    if [ $r = 1 ]; then cs="def prr"; else cs="prr def"; fi
    for c in $cs; do python3 $D/run1.py $c $p 600 $r long600; done
  done
done
echo ALLDONE

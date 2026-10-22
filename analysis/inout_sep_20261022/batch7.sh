#!/bin/bash
# 6 版目: ゲートの判定を x(α) の分離の前に移した版 (止めた求解の経過を既定とそろえる)。ログは logs/b7/
D=$(cd $(dirname "$0") && pwd)
export LOGDIR=$D/logs/b7; mkdir -p $LOGDIR
for k in 1 2; do
  for p in mc11 timtab1 qnet1 mik-250-20-75-4 h80x6320d roll3000 10teams neos-960392 beasleyC3 neos-911970; do $D/erun.sh $p faceg $k; done
  $D/erun.sh physiciansched6-2 e2iog $k
done

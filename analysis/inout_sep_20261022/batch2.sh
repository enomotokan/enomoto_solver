#!/bin/bash
# 対象 5 問 (+ physiciansched6-2 の代入あり) で def / face / io を 3 回ずつ (2 版目: 内点法の時間を上限に数えない・x_lp のカットと平行な候補を先に除く)
D=$(cd $(dirname "$0") && pwd)
for k in 1 2 3; do
  for p in roll3000 mzzv42z neos-911970 mc11 physiciansched6-2; do
    for c in def face io; do $D/erun.sh $p $c $k; done
  done
  for c in e2 e2face e2io; do $D/erun.sh physiciansched6-2 $c $k; done
done

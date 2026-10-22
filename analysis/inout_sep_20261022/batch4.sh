#!/bin/bash
# 3 版目 (診断の表示 + INOUT_NO_BG): カットを強める求解では in-out を使わない facenb / ionb を 3 回ずつ
D=$(cd $(dirname "$0") && pwd)
for k in 1 2 3; do
  for p in mc11 roll3000 neos-911970 mzzv42z physiciansched6-2; do
    for c in facenb ionb; do $D/erun.sh $p $c $k; done
  done
done

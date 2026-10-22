#!/bin/bash
# 1 回目の当たり: 対象 5 問で def / io (最初の中心を使い続ける) / face (毎ラウンド中心を取り直す)
D=$(cd $(dirname "$0") && pwd)
for p in physiciansched6-2 roll3000 mzzv42z neos-911970 mc11; do
  for c in def io face; do $D/erun.sh $p $c 1; done
done
for c in e2 e2io e2face; do $D/erun.sh physiciansched6-2 $c 1; done

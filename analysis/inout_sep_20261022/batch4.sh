#!/bin/bash
# 3 版目 (診断の表示 + INOUT_NO_BG): カットを強める求解では in-out を使わない facenb / ionb を 3 回ずつ
D=$(cd $(dirname "$0") && pwd)
for k in 1 2 3; do
  for p in mc11 roll3000 neos-911970 mzzv42z physiciansched6-2; do
    for c in facenb ionb; do $D/erun.sh $p $c $k; done
  done
done
# 対照: 1 ラウンドの上限を 100 本に (in-out の別枠 50 本の分だけ多く入れるのと比べる)
for k in 1 2; do
  for p in neos-911970 mc11 beasleyC3 roll3000; do $D/erun.sh $p mc100 $k; done
  $D/erun.sh physiciansched6-2 e2mc100 $k
done
# 副作用の重い 2 問を NO_BG で
for k in 1 2; do for p in h80x6320d qnet1 beasleyC3; do $D/erun.sh $p facenb $k; done; done

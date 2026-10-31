#!/bin/bash
# 3: ほかの問題で再スタートが起きるか、再スタート後の根で bgw の待ちが上限に当たるかを見る (bgw と r を 1 回ずつ)
cd "$(dirname "$0")"
for p in eil33-2 neos5 noswot neos17 physiciansched6-2; do
  ./run.sh $p ${p}_bgw_1 bgw
  ./run.sh $p ${p}_r_1 r
done

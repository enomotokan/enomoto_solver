#!/bin/sh
# 1 段目: 7 問それぞれで SCIP の大きな手 (上限 120 秒) (分離・前処理・伝播・ヒューリスティクス・対称性・衝突・再スタート・根の分離 1 巡) を 1 つずつ切る。全部ログつき。
cd "$(dirname "$0")"
for p in exp-1-500-5-5 app1-1 neos-1171448 mc11 beasleyC3 p200x1188c decomp2; do
  python scipab.py "$p" GROUPS --tl 120 --log
done

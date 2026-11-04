#!/bin/sh
# 試作の間隔を決める 60 秒の回 (glass4・icir97_tension、4 スレッド)
D=$(dirname "$0")
for p in glass4 icir97_tension; do
  for c in p3_5 p3_05 p3_005 tk tkp3; do sh $D/run2.sh $p $c 60 4; done
done
echo done > $D/queue2.done

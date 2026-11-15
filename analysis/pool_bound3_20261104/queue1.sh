#!/bin/sh
# 60 秒の分類 (10 問) → supportcase18 の 600 秒 (def・p3) → 5 問の 600 秒 (def・p3)。すべて 4 スレッド・診断つき。
D=$(dirname "$0")
for p in supportcase18 csched008 icir97_tension neos-3024952-loue radiationm18-12-05 traininstance2 glass4 neos-1445765 neos-5195221-niemur uccase9 bppc4-08; do
  sh $D/run_one.sh $p def 60 4
done
for p in supportcase18 csched008 icir97_tension neos-3024952-loue radiationm18-12-05 traininstance2; do
  sh $D/run_one.sh $p def 600 4
  sh $D/run_one.sh $p p3 600 4
done
echo ALLDONE > $D/queue1.done

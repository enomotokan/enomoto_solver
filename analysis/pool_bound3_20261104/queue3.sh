#!/bin/sh
# 600 秒の比較 (既定 def と試作 p3_005 = ENOMOTO_MIP_POOL3_BEST=0.05、4 スレッド・診断つき) → 58 問 (60 秒、診断なし)
D=$(dirname "$0")
sh $D/run2.sh supportcase18 p3_005 600 4
for p in icir97_tension csched008 neos-3024952-loue glass4; do
  sh $D/run2.sh $p def 600 4
  sh $D/run2.sh $p p3_005 600 4
done
python $D/bench.py def,p3
echo done > $D/queue3.done

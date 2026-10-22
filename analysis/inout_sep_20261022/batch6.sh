#!/bin/bash
# 5 版目: 親の 40+16 問で io が落とした 10teams・neos-960392 を gate=2 で、EQ2_ONLY の確認
D=$(cd $(dirname "$0") && pwd)
for k in 1 2; do
  for p in 10teams neos-960392; do for c in def io iog faceg; do $D/erun.sh $p $c $k; done; done
  $D/erun.sh physiciansched6-2 e2ioeq $k
  $D/erun.sh neos-911970 ioeq $k
done

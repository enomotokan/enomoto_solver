#!/bin/bash
# 手元の当たり (README の「手元」節): 効く ns1830653・害の記録のある tr12-30・traininstance6 を既定と試作で 1 回ずつ (種 1、600 秒)
D=$(cd "$(dirname "$0")" && pwd)
for p in ns1830653 tr12-30 traininstance6; do
  for c in def xs; do
    if [ $c = xs ]; then a="ENOMOTO_MIP_NODELP2=xs"; else a=""; fi
    echo "$p $c $(bash $D/one.sh $p ours 1 600 $c $a | cut -c1-220)"
  done
done

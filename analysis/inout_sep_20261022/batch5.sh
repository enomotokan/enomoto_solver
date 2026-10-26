#!/bin/bash
# 4 版目: 最初のラウンドで x(α) の分数の列が x_lp の 2 倍未満なら止める (ENOMOTO_T_MIP_INOUT_GATE=2)
D=$(cd $(dirname "$0") && pwd)
for k in 1 2; do
  for p in neos-911970 mc11 h80x6320d qnet1 mik-250-20-75-4 timtab1 roll3000 beasleyC3 noswot misc07; do
    for c in faceg iog; do $D/erun.sh $p $c $k; done
  done
  for c in e2faceg e2iog; do $D/erun.sh physiciansched6-2 $c $k; done
done

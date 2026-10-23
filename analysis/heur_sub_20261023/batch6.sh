#!/bin/bash
# 改訂 2 の後の組み合わせ: ACS + LB2 + PROX2 (30n20b8・neos-1456979、2 回ずつ)
cd "$(dirname "$0")"
for r in 1 2; do for p in 30n20b8 neos-1456979; do
  ./run_cfg.sh $p 60 acslbproxv3 ENOMOTO_MIP_ACS=1 ENOMOTO_MIP_LB2=1 ENOMOTO_MIP_PROX2=1
done; done

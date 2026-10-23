#!/bin/bash
# 30n20b8・neos-1456979 を 8 つの設定で 2 回ずつ (4 スレッド・60 秒)
cd "$(dirname "$0")"; source configs.sh
for r in 1 2; do for p in 30n20b8 neos-1456979; do for c in base farkas zeroobj acs lb2 prox2 acsprox all5; do
  ./run_cfg.sh $p 60 $c ${CFG[$c]}
done; done; done

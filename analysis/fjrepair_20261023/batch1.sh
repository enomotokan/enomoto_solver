#!/bin/bash
# 1. 数える: 55 問 (40 問 + highsonly16) を 4 スレッド・60 秒で 1 回ずつ、診断 ENOMOTO_MIP_FJ_STATS=1 だけ (既定の動き)
cd "$(dirname "$0")"
P40=$(cd ../../scripts && python3 -c "from run_miplib_benchmark import PROBLEMS; print(' '.join(PROBLEMS))")
P16="nursesched-sprint02 hypothyroid-k1 ns1116954 neos-950242 drayage-25-23 drayage-100-23 beasleyC3 mc11 physiciansched6-2 uccase12 neos-2978193-inde neos-911970 mzzv42z neos-960392 roll3000 app1-1"
for p in $P40 $P16; do
  [ -e logs/${p}_stats_1.log ] && continue
  ./run_cfg.sh $p 60 stats ENOMOTO_MIP_FJ_STATS=1
done
echo ALL DONE

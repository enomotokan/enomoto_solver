#!/bin/bash
# 段階ごとの時間 (1 スレッド・根だけ、と 4 スレッド・根だけ)
cd $(dirname $0)
for P in roll3000 mzzv42z 30n20b8 neos-911970 physiciansched6-2; do
  ./run_eno.sh $P ${P}_t1_prof 60 1 NODE_LIMIT=1 ENOMOTO_MIP_NO_BG_CUTS=1 ENOMOTO_MIP_DEBUG_SEPPROF=1 ENOMOTO_MIP_DEBUG_FACETIME=1
  ./run_eno.sh $P ${P}_t4_prof 60 4 NODE_LIMIT=1 ENOMOTO_MIP_DEBUG_SEPPROF=1 ENOMOTO_MIP_DEBUG_FACETIME=1
done

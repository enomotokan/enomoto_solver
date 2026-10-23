#!/bin/bash
# CMIR の速い評価 (SEP_FAST)、δ の数を減らす (CMIR_DELTAS_HIGHS)、tableau 行の上限 (1 スレッド・根だけ)
cd $(dirname $0)
B="NODE_LIMIT=1 ENOMOTO_MIP_NO_BG_CUTS=1 ENOMOTO_MIP_DEBUG_SEPPROF=1 ENOMOTO_MIP_DEBUG_FACETIME=1"
for P in roll3000 mzzv42z 30n20b8 neos-911970; do
  ./run_eno.sh $P ${P}_t1_fast 60 1 $B ENOMOTO_MIP_SEP_FAST=1
  ./run_eno.sh $P ${P}_t1_fastdh 60 1 $B ENOMOTO_MIP_SEP_FAST=1 ENOMOTO_MIP_CMIR_DELTAS_HIGHS=1
done
./run_eno.sh mzzv42z mzzv42z_t1_tab02 60 1 $B ENOMOTO_MIP_SEP_FAST=1 ENOMOTO_T_MIP_TAB_ROWS_FRAC=0.02

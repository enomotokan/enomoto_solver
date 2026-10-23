#!/bin/bash
# tableau の予算 (TAB_BUDGET) と、試作の組 (P = SEP_FAST + CMIR_DELTAS_HIGHS + FACE_ADAPT + TAB_BUDGET) (1 スレッド・根だけ)
cd $(dirname $0)
B="NODE_LIMIT=1 ENOMOTO_MIP_NO_BG_CUTS=1 ENOMOTO_MIP_DEBUG_SEPPROF=1 ENOMOTO_MIP_DEBUG_FACETIME=1"
P="ENOMOTO_MIP_SEP_FAST=1 ENOMOTO_MIP_CMIR_DELTAS_HIGHS=1 ENOMOTO_MIP_FACE_ADAPT=1"
for X in mzzv42z 30n20b8 roll3000 neos-911970 physiciansched6-2; do
  ./run_eno.sh $X ${X}_t1_def2 60 1 $B
  ./run_eno.sh $X ${X}_t1_tb 60 1 $B ENOMOTO_MIP_TAB_BUDGET=1
  ./run_eno.sh $X ${X}_t1_P 60 1 $B $P ENOMOTO_MIP_TAB_BUDGET=1
done

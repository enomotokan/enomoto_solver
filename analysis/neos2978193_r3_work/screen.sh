#!/bin/bash
# 中間の設定のふるい分け: 60 秒 1 回 (neos-2978193-inde は打ち切り値つき 300 秒)
cd "$(dirname "$0")"
declare -A V=(
  [pq1s]="ENOMOTO_T_MIP_PLUNGE_QUOT=1"
  [sq1]="ENOMOTO_T_MIP_SIBLING_QUOT=1"
  [q05]="ENOMOTO_T_MIP_PLUNGE_QUOT=0.5 ENOMOTO_T_MIP_SIBLING_QUOT=0.5"
)
for v in pq1s sq1 q05; do
  for p in neos-911970 neos5 mas76 qiu mik-250-20-75-4; do
    export MPS=/home/user/enomoto_solver/.miplib_cache/$p.mps
    echo "$p $v 1 $(./run.sh side_${p}_${v}_1 60 eno ${V[$v]} | cut -c1-200)"
  done
  unset MPS
  echo "inde $v 1 $(./run.sh cut_${v}_1 300 eno ENOMOTO_MIP_CUTOFF=-2.38805 ${V[$v]} | cut -c1-200)"
done

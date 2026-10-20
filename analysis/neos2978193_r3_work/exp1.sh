#!/bin/bash
# 打ち切り値つき (証明だけ) 4 スレッド 300 秒、設定ごとに 2 回
cd "$(dirname "$0")"
C="ENOMOTO_MIP_CUTOFF=-2.38805"
run() { n=$1; shift; for i in 1 2; do echo "$n $i $(./run.sh cut_${n}_$i 300 eno $C "$@" | cut -c1-200)"; done; }
run def
run plq1 ENOMOTO_T_MIP_PLUNGE_QUOT=1 ENOMOTO_T_MIP_SIBLING_QUOT=1
run rsol ENOMOTO_MIP_CHILD_ROOTSOL=1
run both ENOMOTO_MIP_CHILD_ROOTSOL=1 ENOMOTO_T_MIP_PLUNGE_QUOT=1 ENOMOTO_T_MIP_SIBLING_QUOT=1

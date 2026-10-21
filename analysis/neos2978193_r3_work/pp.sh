#!/bin/bash
# 試作 ENOMOTO_MIP_PROOF_PLUNGE (検出は停滞と相対ギャップだけ): 担当の問題 (打ち切り値なし 300 秒) と副作用の問題 (60 秒)、各 2 回
cd "$(dirname "$0")"
for i in 1 2; do
  unset MPS
  echo "inde ppg $i $(./run.sh full_ppg_$i 300 eno ENOMOTO_MIP_PROOF_PLUNGE=1 ENOMOTO_T_MIP_PROOF_GAP_FRAC=1 | cut -c1-200)"
  for p in neos-911970 neos5 gen-ip002 timtab1 mas76 qiu mik-250-20-75-4 eil33-2 pk1; do
    export MPS=/home/user/enomoto_solver/.miplib_cache/$p.mps
    echo "$p ppg $i $(./run.sh side_${p}_ppg_$i 60 eno ENOMOTO_MIP_PROOF_PLUNGE=1 ENOMOTO_T_MIP_PROOF_GAP_FRAC=1 | cut -c1-200)"
  done
done
unset MPS
echo "inde ppg 3 $(./run.sh full_ppg_3 300 eno ENOMOTO_MIP_PROOF_PLUNGE=1 ENOMOTO_T_MIP_PROOF_GAP_FRAC=1 | cut -c1-200)"

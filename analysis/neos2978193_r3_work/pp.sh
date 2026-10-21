#!/bin/bash
# 試作 ENOMOTO_MIP_PROOF_PLUNGE: 担当の問題 (打ち切り値なし 300 秒) と副作用の問題 (60 秒)、各 2 回
cd "$(dirname "$0")"
for i in 1 2; do
  unset MPS
  echo "inde pp $i $(./run.sh full_pp_$i 300 eno ENOMOTO_MIP_PROOF_PLUNGE=1 | cut -c1-200)"
  for p in neos-911970 neos5 gen-ip002 timtab1 mas76 qiu mik-250-20-75-4 eil33-2 pk1; do
    export MPS=/home/user/enomoto_solver/.miplib_cache/$p.mps
    echo "$p pp $i $(./run.sh side_${p}_pp_$i 60 eno ENOMOTO_MIP_PROOF_PLUNGE=1 | cut -c1-200)"
  done
done
unset MPS
echo "inde pp 3 $(./run.sh full_pp_3 300 eno ENOMOTO_MIP_PROOF_PLUNGE=1 | cut -c1-200)"

#!/bin/bash
# 副作用の確認: 4 スレッド 60 秒、既定 / 潜りの打ち切りなし、各 2 回
cd "$(dirname "$0")"
PQ="ENOMOTO_T_MIP_PLUNGE_QUOT=1 ENOMOTO_T_MIP_SIBLING_QUOT=1"
for i in 1 2; do
for p in eil33-2 neos-860300 neos-911970 neos5 timtab1 10teams gen-ip002; do
  export MPS=/home/user/enomoto_solver/.miplib_cache/$p.mps
  echo "$p def $i $(./run.sh side_${p}_def_$i 60 eno | cut -c1-200)"
  echo "$p plq1 $i $(./run.sh side_${p}_plq1_$i 60 eno $PQ | cut -c1-200)"
done; done

#!/bin/bash
# 副作用: 既定と試作 3 つを全部有効にした場合を 10 問で 2 回ずつ (4 スレッド・60 秒)
cd /home/user/enomoto_solver/work_ucc; mkdir -p logs/side
PROTO="ENOMOTO_MIP_BG_CUTS_BASIS=1 ENOMOTO_MIP_ROOT_GAP_EXIT=1 ENOMOTO_MIP_FJ_FEAS_EXTEND=1"
for rep in 1 2; do
for n in h80x6320d qnet1 binkar10_1 timtab1 neos-1456979 rout 10teams misc07 nw04 air03; do
  for cfg in def proto; do
    if [ $cfg = def ]; then E=""; else E=$PROTO; fi
    env ENOMOTO_MIP_THREADS=4 $E python run_eno.py ../.miplib_cache/$n.mps 60 > logs/side/${n}_${cfg}_$rep.log 2>&1
    echo "$n $cfg $rep $(grep RESULT logs/side/${n}_${cfg}_$rep.log | cut -c8-)"
  done
done; done

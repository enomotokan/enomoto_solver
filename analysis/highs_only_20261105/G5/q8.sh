#!/bin/bash
cd $(dirname $0)
until grep -q Q58DONE q.log; do sleep 30; done
python3 hsol2.py pre_t6_prr.mps sol_pre_t6_prr.txt 900 > logs/hsol_pre_t6_prr.log 2>&1
./run1.sh ours pre_t6_prr 600 dbgsol ENOMOTO_MIP_TREE4_TRACE=5 ENOMOTO_MIP_DEBUG_SOL=$(pwd)/sol_pre_t6_prr.txt
echo Q8DONE >> q.log
python3 run58.py nofreq > run58b.log 2>&1
echo Q58BDONE >> q.log

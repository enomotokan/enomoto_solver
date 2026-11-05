#!/bin/bash
cd $(dirname $0)
until grep -q Q4DONE q.log; do sleep 20; done
./build.sh
until grep -q Q5DONE q.log; do sleep 20; done
S=$(pwd)/sol_traininstance6.txt
./run1.sh ours traininstance6 120 dbgsol_def ENOMOTO_MIP_DEBUG_SOL=$S
./run1.sh ours traininstance6 120 dbgsol_prr ENOMOTO_MIP_DEBUG_SOL=$S ENOMOTO_MIP_PROBE_REPRESOLVE=1
./run1.sh ours neos-4722843-widden 600 bgstall ENOMOTO_MIP_TREE4_TRACE=5 ENOMOTO_MIP_HO5_BGSTALL=1
./run1.sh ours neos-4722843-widden 600 ho5 ENOMOTO_MIP_TREE4_TRACE=5 ENOMOTO_MIP_HO5_BGSTALL=1 ENOMOTO_MIP_HO5_CUTLP_GAIN=1
./run1.sh ours neos-1456979 600 hstall ENOMOTO_MIP_TREE4_TRACE=5 ENOMOTO_MIP_CUT_STALL_HIGHS=1
./run1.sh ours mcsched 600 nofreq ENOMOTO_MIP_TREE4_TRACE=5 ENOMOTO_T_MIP_NODE_CUT_FREQ=0
echo Q6DONE >> q.log
python3 run58.py def ho5 > run58.log 2>&1
echo Q58DONE >> q.log

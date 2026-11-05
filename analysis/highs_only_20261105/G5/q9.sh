#!/bin/bash
cd $(dirname $0)
while pgrep -f hsol2.py > /dev/null; do sleep 10; done
./run1.sh ours pre_t6_prr 600 dbgsol ENOMOTO_MIP_TREE4_TRACE=5 ENOMOTO_MIP_DEBUG_SOL=$(pwd)/sol_pre_t6_prr.txt
echo Q8DONE >> q.log
python3 run58.py def ho5 nofreq > run58.log 2>&1
echo Q58ALL >> q.log

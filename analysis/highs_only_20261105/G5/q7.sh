#!/bin/bash
cd $(dirname $0)
until grep -q '"hstall"' res.jsonl; do sleep 10; done
./run1.sh ours mcsched 600 nofreq ENOMOTO_MIP_TREE4_TRACE=5 ENOMOTO_T_MIP_NODE_CUT_FREQ=0
R=/home/user/enomoto_solver
for v in def prr; do
  e=""; [ $v = prr ] && e="ENOMOTO_MIP_PROBE_REPRESOLVE=1"
  env $e ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED3=$(pwd)/pre_t6_$v.mps ENOMOTO_MIP_DEBUG_STOP_PRESOLVED=1 python $R/scripts/run_miplib_benchmark.py --worker-ours $R/.miplib_cache/traininstance6.mps --time-limit 60 --threads 4 > logs/dump_t6_$v.log 2>&1
  python3 hcheck.py pre_t6_$v.mps 28390 600 > logs/hcheck_t6_$v.log 2>&1
done
echo Q6DONE >> q.log
python3 run58.py def ho5 > run58.log 2>&1
echo Q58DONE >> q.log

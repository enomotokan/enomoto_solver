#!/bin/bash
R=/home/user/enomoto_solver/runs; mkdir -p $R/logs
for p in beasleyC3 mc11; do
  python $R/run_highs.py $p 60 > $R/logs/highs4_$p.log 2>&1
  $R/go.sh def $p 60
  $R/go.sh fc $p 60 ENOMOTO_MIP_FC_NODE_SEP=1
  $R/go.sh ad $p 60 ENOMOTO_MIP_FC_NODE_SEP_ADAPT=1
done

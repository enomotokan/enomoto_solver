#!/bin/bash
R=/home/user/wt/beasleyC3_mc11/runs
for p in beasleyC3 mc11; do /tmp/with_slot.sh python $R/run_highs.py $p 60 > $R/highs_$p.log 2>&1; tail -n1 $R/highs_$p.log; done
until [ -f /home/user/wt/beasleyC3_mc11/.pylib/enomoto_solver/__init__.py ]; do sleep 10; done
for p in beasleyC3 mc11; do
  $R/go.sh base $p 60
  $R/go.sh fc $p 60 ENOMOTO_MIP_FC_NODE_SEP=1
  $R/go.sh ad $p 60 ENOMOTO_MIP_FC_NODE_SEP_ADAPT=1
done

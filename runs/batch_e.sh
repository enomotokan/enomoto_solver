#!/bin/bash
R=/home/user/enomoto_solver/runs
for p in h80x6320d rout gr4x6 ran14x18-disj-8 beavma; do
  $R/go.sh def $p 60
  $R/go.sh s2k $p 60 ENOMOTO_MIP_FC_NODE_SEP2=1
  python $R/run_highs.py $p 60 > $R/logs/highs4_$p.log 2>&1
done
echo DONE

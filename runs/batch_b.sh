#!/bin/bash
R=/home/user/enomoto_solver/runs
for p in beasleyC3 mc11; do
  $R/go.sh s2 $p 60 ENOMOTO_MIP_FC_NODE_SEP2=1
  $R/go.sh s2h $p 60 ENOMOTO_MIP_FC_NODE_SEP2=1 ENOMOTO_MIP_FC_NODE_PATH_HIGHS=1
done

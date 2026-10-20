#!/bin/bash
R=/home/user/enomoto_solver/runs
$R/go.sh g beasleyC3 60 ENOMOTO_MIP_FC_NODE_SEP2=1
for p in mc11 ran14x18-disj-8 h80x6320d; do
  $R/go.sh def2 $p 60
  $R/go.sh g $p 60 ENOMOTO_MIP_FC_NODE_SEP2=1
done
$R/go.sh g rout 60 ENOMOTO_MIP_FC_NODE_SEP2=1
echo DONE

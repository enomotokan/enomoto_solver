#!/bin/bash
R=/home/user/enomoto_solver/runs
E="ENOMOTO_MIP_FC_NODE_SEP2=1 ENOMOTO_T_MIP_FC_SEP_TIME_FRAC=0.1"
for p in beasleyC3 mc11 ran14x18-disj-8 rout h80x6320d; do $R/go.sh f01 $p 60 $E; done
$R/go.sh f01b beasleyC3 60 $E
$R/go.sh def2 rout 60
$R/go.sh s2k2 rout 60 ENOMOTO_MIP_FC_NODE_SEP2=1
echo DONE

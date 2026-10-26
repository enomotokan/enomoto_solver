#!/bin/bash
cd /home/user/enomoto_solver
P5S="ENOMOTO_MIP_INEQ_SINGLETON=1 ENOMOTO_MIP_DUAL_UB=1 ENOMOTO_MIP_CS_OBJ_PIVOT=1 ENOMOTO_MIP_FJ_LP_REPAIR=1 ENOMOTO_MIP_INC_LP_CAP=1"
for r in 1 2 3 4 5 6; do python runs/bc3r3/run.py fcsub $r 60 ENOMOTO_MIP_FC_SUBMIP=1; done
for r in 1 2 3; do python runs/bc3r3/run.py p5sfcsub $r 60 ENOMOTO_MIP_FC_SUBMIP=1 $P5S; done
for r in 1 2; do for p in rout h80x6320d ran14x18-disj-8 aflow30a mc11 r50x360 beasleyC2; do
  PROB=$p python runs/bc3r3/run.py def $r 60
  PROB=$p python runs/bc3r3/run.py fcsub $r 60 ENOMOTO_MIP_FC_SUBMIP=1
done; done

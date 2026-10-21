#!/bin/bash
cd /home/user/enomoto_solver
P5S="ENOMOTO_MIP_INEQ_SINGLETON=1 ENOMOTO_MIP_DUAL_UB=1 ENOMOTO_MIP_CS_OBJ_PIVOT=1 ENOMOTO_MIP_FJ_LP_REPAIR=1 ENOMOTO_MIP_INC_LP_CAP=1"
python runs/bc3r3/highs.py 1 60 2>&1 | grep HIGHS
for r in 1 2 3 4; do
  python runs/bc3r3/run.py def $r 60
  python runs/bc3r3/run.py p5s $r 60 $P5S
done
python runs/bc3r3/highs.py 2 60 2>&1 | grep HIGHS

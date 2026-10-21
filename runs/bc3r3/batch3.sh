#!/bin/bash
cd /home/user/enomoto_solver
P5S="ENOMOTO_MIP_INEQ_SINGLETON=1 ENOMOTO_MIP_DUAL_UB=1 ENOMOTO_MIP_CS_OBJ_PIVOT=1 ENOMOTO_MIP_FJ_LP_REPAIR=1 ENOMOTO_MIP_INC_LP_CAP=1"
for r in 1 2 3 4 5 6; do python runs/bc3r3/run.py ext $r 60 ENOMOTO_T_MIP_SYM_TIME_EXT=1; done
for r in 1 2 3 4; do python runs/bc3r3/run.py p5sext $r 60 ENOMOTO_T_MIP_SYM_TIME_EXT=1 $P5S; done
for r in 1 2; do for p in 10teams 30n20b8 binkar10_1 nw04 blp-ar98 h80x6320d graph20-20-1rand; do
  PROB=$p python runs/bc3r3/run.py def $r 60
  PROB=$p python runs/bc3r3/run.py ext $r 60 ENOMOTO_T_MIP_SYM_TIME_EXT=1
done; done

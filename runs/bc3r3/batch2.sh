#!/bin/bash
cd /home/user/enomoto_solver
for r in 5 6 7 8; do python runs/bc3r3/run.py symt1 $r 60 ENOMOTO_T_MIP_SYM_TIME=1; done
for r in 5 6; do python runs/bc3r3/run.py def $r 60; done
for r in 1 2; do python runs/bc3r3/run.py defco $r 60 ENOMOTO_MIP_CUTOFF=754.5; python runs/bc3r3/run.py symt1co $r 60 ENOMOTO_T_MIP_SYM_TIME=1 ENOMOTO_MIP_CUTOFF=754.5; done

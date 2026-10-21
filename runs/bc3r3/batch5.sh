#!/bin/bash
cd /home/user/enomoto_solver
for r in 4 5 6 7 8; do
  python runs/bc3r3/run.py sc $r 60 ENOMOTO_MIP_SUBMIP_CUTS=1
  python runs/bc3r3/run.py scr2 $r 60 ENOMOTO_MIP_SUBMIP_CUTS=1 ENOMOTO_T_MIP_SUBMIP_CUT_ROUNDS=25 ENOMOTO_T_MIP_SUBMIP_CUT_STALL=10
  python runs/bc3r3/run.py def $r 60
done

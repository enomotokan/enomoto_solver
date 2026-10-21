#!/bin/bash
cd "$(dirname "$0")"
C="ENOMOTO_MIP_CUTOFF=-2.38805 ENOMOTO_MIP_FLAT_RESTART=1"
for i in 1 2 3; do
./run.sh co_fr_$i 300 $C
./run.sh co_nosb_$i 300 $C ENOMOTO_T_MIP_SB_MAXCAND=0
done

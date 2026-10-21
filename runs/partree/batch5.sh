#!/bin/bash
cd "$(dirname "$0")"
while pgrep -f batch4.sh >/dev/null; do sleep 5; done
S="ENOMOTO_MIP_SUBMIP_CUTS=1 ENOMOTO_T_MIP_SUBMIP_CUT_ROUNDS=25 ENOMOTO_T_MIP_SUBMIP_CUT_STALL=10"
D="ENOMOTO_MIP_TREE_DIAG=1 ENOMOTO_MIP_HEUR_ROLE_VERBOSE=1"
./loop.sh newS mc11 20 4 60 $D $S
./loop.sh newSfc mc11 10 4 60 $D $S ENOMOTO_MIP_FC_SUBMIP=1
./loop.sh newS beasleyC3 10 4 60 $D $S
./loop.sh newdef beasleyC3 10 4 60 $D
./loop.sh newS rout 15 4 60 $D $S
./loop.sh newdef rout 15 4 60 $D
./loop.sh newS neos-911970 10 4 60 $D $S
./loop.sh newdef neos-911970 10 4 60 $D
# CPU を取り合わせる: 4 スレッドの求解を 2 つ重ねる
./loop.sh newcpuA mc11 8 4 60 $D $S &
./loop.sh newcpuB rout 25 4 60 $D $S
wait

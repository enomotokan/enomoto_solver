#!/bin/bash
cd "$(dirname "$0")"
for i in 2 3; do
./run.sh base300_$i 300
./run.sh fr300_$i 300 ENOMOTO_MIP_FLAT_RESTART=1
./run.sh frf3_300_$i 300 ENOMOTO_MIP_FLAT_RESTART=1 ENOMOTO_T_MIP_FLAT_CUT_ROUNDS=3
done

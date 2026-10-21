#!/bin/bash
cd "$(dirname "$0")"
for i in 2 3 4 5 6; do
./run.sh base60_$i 60
./run.sh fr60_$i 60 ENOMOTO_MIP_FLAT_RESTART=1
./run.sh frr3_60_$i 60 ENOMOTO_MIP_FLAT_RESTART=1 ENOMOTO_MIP_RESTORE_READD=3
done

#!/bin/bash
# q600.sh の後に回す: csched008 の追加の 2 回と 58 問・60 秒 (既定と ho2)
cd "$(dirname "$0")"
while pgrep -f q600.sh > /dev/null; do sleep 10; done
H="ENOMOTO_MIP_HO2_IMPLINT=1 ENOMOTO_MIP_HO2_GOBJ=1"
./run1.sh ho2trr csched008 600 $H ENOMOTO_MIP_TREE_RESTART=1 ENOMOTO_MIP_TREE_RESTART_LOG=1 ENOMOTO_MIP_RESTART_CUTS_READD=1
./run1.sh ho2tr_r2 csched008 600 $H ENOMOTO_MIP_TREE_RESTART=1 ENOMOTO_MIP_TREE_RESTART_LOG=1
python run58.py 1 def ho2 > run58.log 2>&1

#!/bin/bash
# q2.sh の後: swath3 の揺れを見る追加の回
cd "$(dirname "$0")"
while pgrep -f "q2.sh|q600.sh" > /dev/null; do sleep 10; done
H="ENOMOTO_MIP_HO2_IMPLINT=1 ENOMOTO_MIP_HO2_GOBJ=1"
./run1.sh def_r2 swath3 600
./run1.sh ho2tr_r2 swath3 600 $H ENOMOTO_MIP_TREE_RESTART=1 ENOMOTO_MIP_TREE_RESTART_LOG=1
./run1.sh def_r3 swath3 600

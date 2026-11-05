#!/bin/bash
# q3.sh の後: csched008 で再スタート前の暫定解を参照解として渡す (既存の試作 ENOMOTO_MIP_RESTART_REF)
cd "$(dirname "$0")"
while pgrep -f "q2.sh|q3.sh|q600.sh" > /dev/null; do sleep 10; done
H="ENOMOTO_MIP_HO2_IMPLINT=1 ENOMOTO_MIP_HO2_GOBJ=1"
./run1.sh ho2trref csched008 600 $H ENOMOTO_MIP_TREE_RESTART=1 ENOMOTO_MIP_TREE_RESTART_LOG=1 ENOMOTO_MIP_RESTART_REF=1

#!/bin/bash
# q5.sh の後: swath3 の ho2 (木の中の再スタートなし) の 2 回目
cd "$(dirname "$0")"
while pgrep -f "q2.sh|q3.sh|q4.sh|q5.sh|q600.sh" > /dev/null; do sleep 10; done
./run1.sh ho2_r2 swath3 600 ENOMOTO_MIP_HO2_IMPLINT=1 ENOMOTO_MIP_HO2_GOBJ=1

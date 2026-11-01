#!/bin/bash
# mc11: 既定・試作 (WAIT)、それぞれ SUBMIP_HIGHS=s あり・なしで 3 回ずつ (交互に並べる)
cd "$(dirname "$0")"
W=ENOMOTO_MIP_BG_CUTS_WAIT=1; S=ENOMOTO_MIP_SUBMIP_HIGHS=s
for i in 1 2 3; do
  ./run.sh mc11 mc11_def_$i
  ./run.sh mc11 mc11_w_$i $W
  ./run.sh mc11 mc11_s_$i $S
  ./run.sh mc11 mc11_ws_$i $W $S
done

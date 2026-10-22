#!/bin/bash
# §2 の 1 ラウンドの比較 (roll3000、HiGHS のラウンド k の LP から): step_all.sh > step_all.txt
D=$(cd $(dirname $0) && pwd)
H=ENOMOTO_MIP_CUTSEL_DCD_SOL=$D/sols/roll3000_highs.sol
E=ENOMOTO_MIP_CUTSEL_DCD_SOL=$D/sols/roll3000_eno_root.sol
for k in 3 6 10 20; do for m in only cand none; do
  $D/step.sh $k $m
  for s in $H $E; do
    for w in 0.5 1; do $D/step.sh $k $m ENOMOTO_MIP_CUTSEL_DCD=$w $s; done
    for w in 0.5 1; do $D/step.sh $k $m ENOMOTO_MIP_CUTSEL_NORM=1 ENOMOTO_MIP_CUTSEL_DCD=$w $s; done
  done
  $D/step.sh $k $m ENOMOTO_MIP_CUTSEL_NORM=1
done; done

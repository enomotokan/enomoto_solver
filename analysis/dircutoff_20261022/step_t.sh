#!/bin/bash
# 1 ラウンドの比較: t = DCD/|x*-x| を効き目の尺度で足す (ENOMOTO_MIP_CUTSEL_DCD_T): step_t.sh > step_t.txt
D=$(cd $(dirname $0) && pwd)
for k in 3 6 10 20; do for m in only none; do for s in highs eno_root; do
  S=ENOMOTO_MIP_CUTSEL_DCD_SOL=$D/sols/roll3000_$s.sol
  for w in 1 3 10; do $D/step.sh $k $m ENOMOTO_MIP_CUTSEL_DCD_T=1 ENOMOTO_MIP_CUTSEL_DCD=$w $S; done
  $D/step.sh $k $m ENOMOTO_MIP_CUTSEL_DCD_T=raw ENOMOTO_MIP_CUTSEL_DCD=1 $S
done; done; done

#!/bin/bash
# 1 ラウンドの比較 (積の形 ENOMOTO_MIP_CUTSEL_DCD_PROD、α = 0.5 と 0): step_prod.sh > step_prod.txt
D=$(cd $(dirname $0) && pwd)
for k in 3 6 10 20; do for m in only cand none; do
  for s in highs eno_root; do for a in 0.5 0; do
    $D/step.sh $k $m ENOMOTO_MIP_CUTSEL_DCD_PROD=1 ENOMOTO_T_CUTSEL_DCD_DENS_EXP=$a ENOMOTO_MIP_CUTSEL_DCD_SOL=$D/sols/roll3000_$s.sol
  done; done
done; done

#!/bin/bash
# roll3000、4 スレッド・60 秒: 最初から暫定解を与える (ENOMOTO_MIP_START_FILE) × (DCD なし / DCD=1 / 積 α=0) × 2 回
D=$(cd $(dirname $0) && pwd)
for i in 1 2; do for s in eno_t6.6 eno_root highs; do
  S=ENOMOTO_MIP_START_FILE=$D/sols/roll3000_$s.sol
  $D/run_eno.sh roll3000 st_${s}_none_$i 60 4 $S
  $D/run_eno.sh roll3000 st_${s}_dcd_$i 60 4 $S ENOMOTO_MIP_CUTSEL_DCD=1
  $D/run_eno.sh roll3000 st_${s}_prod0_$i 60 4 $S ENOMOTO_MIP_CUTSEL_DCD_PROD=1 ENOMOTO_T_CUTSEL_DCD_DENS_EXP=0
done; done

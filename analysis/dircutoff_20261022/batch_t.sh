#!/bin/bash
# 4 スレッド・60 秒: 13 問 × (暫定解がない間は既定の和の形 / t の形) × 2 回、と回し直し 1 本
D=$(cd $(dirname $0) && pwd)
$D/run_eno.sh roll3000 st_eno_t6.6_prod0_1 60 4 ENOMOTO_MIP_START_FILE=$D/sols/roll3000_eno_t6.6.sol ENOMOTO_MIP_CUTSEL_DCD_PROD=1 ENOMOTO_T_CUTSEL_DCD_DENS_EXP=0
for i in 1 2; do
  $D/batch60.sh noincdef_$i ENOMOTO_MIP_CUTSEL_DCD=1 ENOMOTO_MIP_CUTSEL_DCD_NOINC_DEF=1
  $D/batch60.sh t1_$i ENOMOTO_MIP_CUTSEL_DCD=1 ENOMOTO_MIP_CUTSEL_DCD_T=1
done

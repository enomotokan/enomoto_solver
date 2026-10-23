#!/bin/bash
# 4 スレッド・60 秒 (サブ MIP では DCD を使わない版): 13 問 × (DCD=1 / 暫定解がない間は既定 / t の形 / t の形 + 裏のループに暫定解) × 2 回
D=$(cd $(dirname $0) && pwd)
for i in 1 2; do
  $D/batch60.sh v2dcd_$i ENOMOTO_MIP_CUTSEL_DCD=1
  $D/batch60.sh v2noinc_$i ENOMOTO_MIP_CUTSEL_DCD=1 ENOMOTO_MIP_CUTSEL_DCD_NOINC_DEF=1
  $D/batch60.sh v2t1_$i ENOMOTO_MIP_CUTSEL_DCD=1 ENOMOTO_MIP_CUTSEL_DCD_T=1
  $D/batch60.sh v2t1sh_$i ENOMOTO_MIP_CUTSEL_DCD=1 ENOMOTO_MIP_CUTSEL_DCD_T=1 ENOMOTO_MIP_BG_CUTS_SHARE_INC=1
done

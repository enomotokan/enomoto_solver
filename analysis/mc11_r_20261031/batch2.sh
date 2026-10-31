#!/bin/bash
# 2: mc11 を hs_pi0_bgw で、裏のカットのログ (ENOMOTO_MIP_BG_CUTS_LOG) 付きで 4 回
cd "$(dirname "$0")"
for i in 1 2 3 4; do
  ./run.sh mc11 mc11_bgwlog_$i bgw ENOMOTO_MIP_BG_CUTS_LOG=1
done

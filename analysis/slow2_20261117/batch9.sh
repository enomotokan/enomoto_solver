#!/bin/bash
# VB_IMPL (プロービングの含意を連続列の変数上下限に足す) の 600 秒、unitcal_7 種 1・2 (+ 選び方の緩和との組)
D=$(cd "$(dirname "$0")" && pwd)
for s in 1 2; do
  "$D/one.sh" unitcal_7 ours $s 600 vb ENOMOTO_MIP_VB_IMPL=1
  "$D/one.sh" unitcal_7 ours $s 600 vbis ENOMOTO_MIP_VB_IMPL=1 ENOMOTO_MIP_IMPLBD_SELECT=1 ENOMOTO_T_CUTSEL_MIN_REL=0.1 ENOMOTO_T_MIP_ROOT_MAX_CUTS=200 ENOMOTO_T_MIP_CUT_ROUNDS=60 ENOMOTO_MIP_CUT_PHASE2=1
done

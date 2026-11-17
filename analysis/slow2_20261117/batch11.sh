#!/bin/bash
# vbis の部品の切り分け: vbis から 1 つずつ外す (unitcal_7 種 1・2)、連続列の多い 3 問で def・vb・vbis (種 1・2)。600 秒
D=$(cd "$(dirname "$0")" && pwd)
declare -A P=([VB]="ENOMOTO_MIP_VB_IMPL=1" [IS]="ENOMOTO_MIP_IMPLBD_SELECT=1" [MR]="ENOMOTO_T_CUTSEL_MIN_REL=0.1" [MC]="ENOMOTO_T_MIP_ROOT_MAX_CUTS=200" [RD]="ENOMOTO_T_MIP_CUT_ROUNDS=60" [P2]="ENOMOTO_MIP_CUT_PHASE2=1")
ALL="${P[VB]} ${P[IS]} ${P[MR]} ${P[MC]} ${P[RD]} ${P[P2]}"
for s in 1 2; do
  for k in VB IS MR MC RD P2; do
    "$D/one.sh" unitcal_7 ours $s 600 vbis-$k ${ALL/${P[$k]}/}
  done
done
for s in 1 2; do for p in mc11 h80x6320d neos-911970; do
  "$D/one.sh" $p ours $s 600 def
  "$D/one.sh" $p ours $s 600 vb ${P[VB]}
  "$D/one.sh" $p ours $s 600 vbis $ALL
done; done

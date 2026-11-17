#!/bin/bash
# VB_IMPL の害の手当て: 根でだけ (vbr = VB_IMPL=r) と強さで選ぶ (vbm = MINREL 0.25)。M で全種遅かった 3 問 + unitcal_7 の根の下界。600 秒
D=$(cd "$(dirname "$0")" && pwd)
for s in 1 2; do for p in neos-4738912-atrato neos-1456979; do
  "$D/one.sh" $p ours $s 600 def
  "$D/one.sh" $p ours $s 600 vb ENOMOTO_MIP_VB_IMPL=1
  "$D/one.sh" $p ours $s 600 vbr ENOMOTO_MIP_VB_IMPL=r
  "$D/one.sh" $p ours $s 600 vbm ENOMOTO_MIP_VB_IMPL=1 ENOMOTO_T_MIP_VB_IMPL_MINREL=0.25
done; done
for s in 1 2; do
  python "$D/root.py" unitcal_7 r4vbr_s$s ENOMOTO_T_MIP_SEED=$s ENOMOTO_MIP_VB_IMPL=r
  python "$D/root.py" unitcal_7 r4vbm_s$s ENOMOTO_T_MIP_SEED=$s ENOMOTO_MIP_VB_IMPL=1 ENOMOTO_T_MIP_VB_IMPL_MINREL=0.25
done
"$D/one.sh" traininstance6 ours 1 600 def
"$D/one.sh" traininstance6 ours 1 600 vbr ENOMOTO_MIP_VB_IMPL=r
"$D/one.sh" traininstance6 ours 1 600 vbm ENOMOTO_MIP_VB_IMPL=1 ENOMOTO_T_MIP_VB_IMPL_MINREL=0.25

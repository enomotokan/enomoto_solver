#!/bin/bash
# SUBWAIT (根の並列のサブ MIP の待ちの上限 = k x 根の段の秒) の手元の効き目。600 秒、種 1・2
D=$(cd "$(dirname "$0")" && pwd)
for s in 1 2; do for p in h80x6320d hypothyroid-k1 swath1; do
  "$D/one.sh" $p ours $s 600 def
  "$D/one.sh" $p ours $s 600 sw1 ENOMOTO_T_MIP_SUBWAIT=1
  "$D/one.sh" $p ours $s 600 sw05 ENOMOTO_T_MIP_SUBWAIT=0.5
done; done
for s in 1 2; do
  "$D/one.sh" unitcal_7 ours $s 600 sw1 ENOMOTO_T_MIP_SUBWAIT=1
  "$D/one.sh" unitcal_7 ours $s 600 sw05 ENOMOTO_T_MIP_SUBWAIT=0.5
done
"$D/one.sh" tbfp-network ours 1 600 sw1 ENOMOTO_T_MIP_SUBWAIT=1

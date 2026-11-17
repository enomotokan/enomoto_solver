#!/bin/bash
# P で vbis が遅かった問題・速かった問題で、VB_IMPL だけ (vb) と選び方の変更だけ (sel) を既定と比べる。60 秒、種 1・2
D=$(cd "$(dirname "$0")" && pwd)
while pgrep -f batch11.sh >/dev/null; do sleep 20; done
SEL="ENOMOTO_MIP_IMPLBD_SELECT=1 ENOMOTO_T_CUTSEL_MIN_REL=0.1 ENOMOTO_T_MIP_ROOT_MAX_CUTS=200 ENOMOTO_T_MIP_CUT_ROUNDS=60 ENOMOTO_MIP_CUT_PHASE2=1"
for s in 1 2; do for p in neos-960392 qnet1 neos17 h80x6320d timtab1 fiber 10teams p200x1188c blp-ar98 hypothyroid-k1 neos-3083819-nubu; do
  "$D/one.sh" $p ours $s 60 p60def
  "$D/one.sh" $p ours $s 60 p60vb ENOMOTO_MIP_VB_IMPL=1
  "$D/one.sh" $p ours $s 60 p60sel $SEL
done; done

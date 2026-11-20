#!/bin/bash
# cutlen: 群 3 の「長い方が良い」例 (mc11・beasleyC3) で、def・cl・r50 × 種 1・2、60 秒
cd "$(dirname "$0")"
for s in 1 2; do for p in mc11 beasleyC3; do
  ./one.sh $p ours $s 60 def
  ./one.sh $p ours $s 60 cl ENOMOTO_MIP_CUTLEN=1
  ./one.sh $p ours $s 60 r50 ENOMOTO_T_MIP_CUT_ROUNDS=50 ENOMOTO_T_MIP_CUT_TIME_FRAC=0.2
done; done

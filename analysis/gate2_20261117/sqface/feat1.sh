#!/bin/bash
# 既定で 30 秒、根の分離の段階別の時間 (SEPPROF) を集める (FACE_ADAPT が効く問題・遅れる問題の比べ)
D=$(cd "$(dirname "$0")" && pwd)
for p in 10teams roll3000 mc11 air03 h80x6320d rout noswot qiu ns1116954 neos-911970 sp150x300d nw04 mas76; do
  "$D/one.sh" $p ours 1 30 sepprof ENOMOTO_MIP_DEBUG_SEPPROF=1
done

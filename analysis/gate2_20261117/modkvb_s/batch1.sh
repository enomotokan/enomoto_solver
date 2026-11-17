#!/bin/bash
# 今の既定 (0.1.9 + rtc + LPINF) の上で旧試作 MODKVB=1 の効き目と害を代表問題で当て直す (60 秒、種 1・2)
S=$(cd "$(dirname "$0")" && pwd)
for seed in 1 2; do
for p in physiciansched6-2 misc07 h80x6320d mc11 roll3000 noswot neos-860300 swath1; do
  [ -f $S/logs/${p}_def_s$seed.log ] && grep -q '"status"' $S/logs/${p}_def_s$seed.log || bash $S/one.sh $p ours $seed 60 def
  [ -f $S/logs/${p}_mk_s$seed.log ] && grep -q '"status"' $S/logs/${p}_mk_s$seed.log || bash $S/one.sh $p ours $seed 60 mk ENOMOTO_MIP_MODKVB=1
done; done

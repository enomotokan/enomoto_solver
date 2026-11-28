#!/bin/bash
# 試作 ENOMOTO_MIP_PAR4ROLE を、3 節の 13 問の 4T で prof.py にかける: bash batch2.sh <値> >> batch2.out (出力は logs/<問題>_t4_<値>.*)
# 済んだもの (ログに「solve returned」がある) は飛ばす
cd "$(dirname "$0")"
V=$1
P4="nu25-pr12 exp-1-500-5-5 neos-1122047 neos-1171448 mzzv42z dano3_3 mc11 physiciansched6-2 swath3 supportcase7 mcsched CMS750_4 netdiversion"
for p in $P4; do
  [ -f logs/${p}_t4_$V.log.gz ] && continue
  grep -q "solve returned" logs/${p}_t4_$V.log 2>/dev/null && continue
  PROF_TAG=_$V PROF_ENV="ENOMOTO_PROF_TREE_PAR=1 ENOMOTO_MIP_PAR4ROLE=$V" python3 prof.py $p 4 600
done
echo BATCH2_DONE

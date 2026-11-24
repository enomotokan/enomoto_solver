#!/bin/bash
# 根で最適面が広い問題 (analysis/mpsep_20261106.md 6.4 節の 58 問の 13 問 + neos859080) を、既定・CUTVERT=q・CUTROBUST=g、
# 出発の頂点 0 (既定) と 2 で回す (1 スレッド・種 1・60 秒、TIE_FEWER=1)
for p in neos859080 noswot 10teams gt2 pk1 markshare_4_0 neos-911970 neos-960392 neos-827175 neos-3083819-nubu supportcase26 bppc4-08 drayage-25-23 drayage-100-23; do
 for s in 0 2; do
  for c in "def" "q ENOMOTO_MIP_CUTVERT=q" "g ENOMOTO_MIP_CUTROBUST=g"; do set -- $c
   python3 one.py $p w_${1}_start$s ENOMOTO_MIP_CUTSEL_PROBE_TIE_FEWER=1 ENOMOTO_MIP_MPSEP_START=$s $2 | python3 -c "import json,sys;r=json.loads(sys.stdin.read());print(r['problem'],r['tag'],len(r['rounds']),r.get('result',['?']))"
  done
 done
done

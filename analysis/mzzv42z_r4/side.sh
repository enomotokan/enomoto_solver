#!/bin/bash
# 副作用: 問題ごとに既定と試作を交互に解く (4 スレッド・60 秒)。CFGS="def <名前>" と CFGENV="<環境変数...>" で試作を選ぶ (既定は gr = ENOMOTO_MIP_GR_RESTART=1)。使い方: side.sh <回数> <問題...>
D=$(cd $(dirname "$0") && pwd)
n=$1; shift
for i in $(seq 1 $n); do
  for pr in "$@"; do
    for cfg in ${CFGS:-def gr}; do
      if [ $cfg = def ]; then ev="ENOMOTO_MIP_DUMMY=1"; else ev="${CFGENV:-ENOMOTO_MIP_GR_RESTART=1}"; fi
      env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 $ev python3 $D/run_eno.py /home/user/enomoto_solver/.miplib_cache/$pr.mps ${T:-60} > $D/logs/s_${pr}_${cfg}_$i.log 2>&1
      echo "$pr $cfg $i $(grep RESULT $D/logs/s_${pr}_${cfg}_$i.log) restarts $(grep -c '^MIP: restart [0-9]' $D/logs/s_${pr}_${cfg}_$i.log)"
    done
  done
done

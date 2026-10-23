#!/bin/bash
# fire_scan.sh: 各問題を ENOMOTO_MIP_CUT_INTOBJ_STALL=1 (4 スレッド・60 秒の設定) で根のカットのループが終わるまで動かし、切り替えが働いたかを出す
cd "$(dirname "$0")/../heur_ls_20261023"
mkdir -p ../10teams_20261023/scan
for mps in ../../.miplib_cache/*.mps; do
  p=$(basename $mps .mps); L=../10teams_20261023/scan/$p.log
  ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_CUT_INTOBJ_STALL=1 timeout 70 python3 eno_run.py $mps 60 4 > $L 2>&1 &
  pid=$!
  for k in $(seq 1 120); do
    sleep 0.5
    grep -q "^Root cuts:\|cut loop done" $L && break
    kill -0 $pid 2>/dev/null || break
  done
  sleep 0.2; pkill -f "eno_run.py $mps" ; wait $pid 2>/dev/null
  echo "$p $(grep -c 'integral objective, LP bound unchanged' $L) $(grep -m1 '^Root cuts:' $L)"
done

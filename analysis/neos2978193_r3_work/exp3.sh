#!/bin/bash
# 打ち切り値なし 4 スレッド 300 秒: 既定 / 潜りの打ち切りなし / それ + 子の選び方。HiGHS の CPU 時間も見る
cd "$(dirname "$0")"
PQ="ENOMOTO_T_MIP_PLUNGE_QUOT=1 ENOMOTO_T_MIP_SIBLING_QUOT=1"
python cputime.py HIGHS_CPU python highs_run.py /home/user/mps/neos-2978193-inde.mps.gz 60 2>&1 | grep -E "RESULT|HIGHS_CPU"
python cputime.py ENO_CPU env ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_CUTOFF=-2.38805 python eno_run.py /home/user/mps/neos-2978193-inde.mps.gz 300 4 2>&1 | grep -E "^\{|ENO_CPU" | cut -c1-120
for i in 1 2 3; do
  echo "full_def $i $(./run.sh full_def_$i 300 eno | cut -c1-200)"
  echo "full_plq1 $i $(./run.sh full_plq1_$i 300 eno $PQ | cut -c1-200)"
  echo "full_both $i $(./run.sh full_both_$i 300 eno $PQ ENOMOTO_MIP_CHILD_ROOTSOL=1 | cut -c1-200)"
done

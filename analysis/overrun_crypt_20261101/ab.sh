#!/bin/bash
# 使い方: ab.sh 裏の処理の本数 回数 問題...
# 直した形 (on) と ENOMOTO_MIP_NO_FJ_P3_DEADLINE=1 (off、直す前と同じ) を交互に、CPU を食う裏の処理 (hog.sh) の下で回す。
# まとめは /tmp/rep/ab{本数}_on_summary.txt・ab{本数}_off_summary.txt
cd /home/user/enomoto_solver
D=analysis/overrun_crypt_20261101
H=$1; N=$2; shift 2
for i in $(seq 0 $((N-1))); do
  for P in "$@"; do
    $D/hog.sh $H $D/watch.sh $P 60 1 ab${H}_on_r$i
    $D/hog.sh $H $D/watch.sh $P 60 1 ab${H}_off_r$i ENOMOTO_MIP_NO_FJ_P3_DEADLINE=1
  done
done

#!/bin/bash
# 1 問: HiGHS の前処理後と元の問題の形・LP 緩和の値を出す (ENOMOTO 側は eone.sh)。使い方: one.sh <mps> <作業場所>
# HiGHS の書き出す MPS は目的の定数を含む。
mps=$1; W=$2; p=$(basename $mps .mps); D=$(cd $(dirname "$0") && pwd)
ho=$(timeout 600 python3 $D/hpre.py $mps $W/${p}_h.mps 2>&1 | grep HSTAT)
ht=$(echo "$ho" | sed "s/.* t=\([^ ]*\).*/\1/")
o=$(python3 $D/pstats.py $mps)
h=$( [ -f $W/${p}_h.mps ] && python3 $D/pstats.py $W/${p}_h.mps || echo null)
echo "{\"name\":\"$p\",\"hstatus\":\"$(echo $ho | sed 's/.*status=\([^ ]*\).*/\1/')\",\"ht\":\"$ht\",\"orig\":$o,\"h\":$h}"

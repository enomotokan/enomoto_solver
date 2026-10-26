#!/bin/bash
# 4 スレッド・60 秒、試作だけを 2 回ずつ (既定は bench.sh の分を使う): bench1.sh <名前> <試作の環境変数...>
cd $(dirname $0)
TAG=$1; shift
PS=${PS:-"roll3000 mzzv42z 30n20b8 neos-911970 physiciansched6-2 misc07 mik-250-20-75-4 qnet1 10teams neos5 h80x6320d beasleyC3 mc11"}
for rep in 1 2; do
  for X in $PS; do
    ./run_eno.sh $X b_${X}_${TAG}_$rep 60 4 ENOMOTO_MIP_DEBUG_FACETIME=1 "$@"
  done
done

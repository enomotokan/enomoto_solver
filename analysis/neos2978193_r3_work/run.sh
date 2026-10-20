#!/bin/bash
# 1 回の実行: run.sh <名前> <秒> eno|highs [環境変数 A=1 ...] (ENOMOTO) / [option=値 ...] (HiGHS)
# ログは logs/<名前>.log (標準出力+標準エラー。実行中は一時ディレクトリに書き、終わってから移す)
cd "$(dirname "$0")"
name=$1; T=$2; kind=$3; shift 3
MPS=${MPS:-/home/user/mps/neos-2978193-inde.mps.gz}
TMPD=${TMPD:-/tmp/claude-0/-home-user-enomoto-solver/ca10c575-40dd-52be-8176-c82d8c84d523/scratchpad/runlogs}; mkdir -p $TMPD
if [ "$kind" = eno ]; then
  env ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 "$@" python eno_run.py $MPS $T 4 > $TMPD/$name.log 2>&1
else
  python highs_run.py $MPS $T output_flag=True "$@" > $TMPD/$name.log 2>&1
fi
mv $TMPD/$name.log logs/$name.log
tail -1 logs/$name.log

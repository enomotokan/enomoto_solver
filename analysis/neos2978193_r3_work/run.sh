#!/bin/bash
# 1 回の実行: run.sh <名前> <秒> eno|highs [環境変数 A=1 ...] (ENOMOTO) / [option=値 ...] (HiGHS)
# ログは logs/<名前>.log (標準出力+標準エラー)
cd "$(dirname "$0")"
name=$1; T=$2; kind=$3; shift 3
MPS=/home/user/mps/neos-2978193-inde.mps.gz
if [ "$kind" = eno ]; then
  env ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 "$@" python eno_run.py $MPS $T 4 > logs/$name.log 2>&1
else
  python highs_run.py $MPS $T output_flag=True "$@" > logs/$name.log 2>&1
fi
tail -1 logs/$name.log

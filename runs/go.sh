#!/bin/bash
# go.sh <tag> <prob> <T> [ENV=VAL ...]  -> runs/logs/<tag>_<prob>.log (ENOMOTO、4 スレッド、インストール済みのビルド。ログは logs_4t_20261020.tar.gz に固めた)
R=/home/user/enomoto_solver/runs; mkdir -p $R/logs; tag=$1; p=$2; T=$3; shift 3
env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 "$@" python $R/run_eno.py $p $T > $R/logs/${tag}_$p.log 2>&1
grep -E "^RESULT" $R/logs/${tag}_$p.log | cut -c1-200

#!/bin/bash
# run_cfg.sh <問題 or MPS のパス> <秒> <名前> [スレッド] -- [環境変数=値 ...]  -> logs/<問題>_<名前>_<n>.log
# 問題名なら .miplib_cache/<問題>.mps。環境変数 TH でスレッド数 (既定 4)
cd "$(dirname "$0")"; mkdir -p logs sub
p=$1; T=$2; name=$3; shift 3
if [ -f "$p" ]; then mps=$p; p=$(basename $p .mps); else mps=../../.miplib_cache/$p.mps; fi
th=${TH:-4}
n=1; while [ -e logs/${p}_${name}_${n}.log ]; do n=$((n+1)); done
env ENOMOTO_MIP_THREADS=$th ENOMOTO_MIP_LOG=1 "$@" python3 eno_run.py $mps $T $th > logs/${p}_${name}_${n}.log 2>&1
echo "$p $name $n $(tail -1 logs/${p}_${name}_${n}.log)"

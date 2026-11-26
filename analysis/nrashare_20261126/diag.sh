#!/bin/bash
# 4 スレッドの nra の統計のスレッドごとの食い違い: bash analysis/nrashare_20261126/diag.sh <python> 問題 秒 種 名前 [環境変数=値 ...]
# c2c (analysis/combo2_20261125/cfg.py の C2C) に NRA_TRACE=500 とログをつけて 4 スレッドで 1 回解き、stderr を logs_<問題>_<名前>_s<種>.err に残す (コミットしない)
cd "$(dirname "$0")/../.."
PY=$1; P=$2; TL=$3; SEED=$4; NAME=$5; shift 5
E=$(python3 -c "import sys; sys.path.insert(0,'analysis/combo2_20261125'); from cfg import C2C; print(' '.join(f'{k}={v}' for k,v in C2C.items()))")
env $E "$@" ENOMOTO_MIP_NRA_TRACE=500 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SEED=$SEED $PY scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$P.mps --time-limit $TL --threads 4 2> analysis/nrashare_20261126/logs_${P}_${NAME}_s$SEED.err | tail -1

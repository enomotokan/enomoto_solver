#!/bin/bash
# 1 問を 1 スレッドで当方 (ENOMOTO_MIP_LOG=1) と SCIP (ENOMOTO_BENCH_SCIP_LOG=1) で解き、logs/ にログ、res1.jsonl に結果を足す。
# 使い方: bash run1.sh 問題 秒 [設定名 環境変数=値 ...]  (設定名を省くと def と scip の両方)
WT=$(cd "$(dirname "$0")/../.." && pwd); D=$WT/analysis/slowm_20261119; P=$1; TL=$2; shift 2
MPS=$WT/.miplib_cache/$P.mps
one() { name=$1; shift; env "$@" timeout $((TL+300)) $WT/.venv/bin/python $WT/scripts/run_miplib_benchmark.py $W $MPS --time-limit $TL --threads 1 > $D/logs/${P}_${name}.out 2> $D/logs/${P}_${name}.log
  out=$(grep '^{' $D/logs/${P}_${name}.out | tail -1)
  echo "{\"problem\": \"$P\", \"cfg\": \"$name\", \"result\": ${out:-null}}" >> $D/res1.jsonl; echo "$P $name ${out:0:160}"; }
if [ $# -eq 0 ]; then
  W=--worker-ours one def ENOMOTO_MIP_THREADS=1 ENOMOTO_MIP_LOG=1
  W=--worker-scip one scip ENOMOTO_BENCH_SCIP_LOG=1
else
  name=$1; shift; W=--worker-ours one $name ENOMOTO_MIP_THREADS=1 ENOMOTO_MIP_LOG=1 "$@"
fi

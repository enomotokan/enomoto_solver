#!/bin/bash
# 追加の A/A: bash batch2.sh TAG REPS THREADS 問題,... [K=V ...] (普通の機械。ログ logs/<問題>_t<スレッド>_<TAG><回>.log)
cd "$(dirname "$0")"; WT=$(cd ../.. && pwd); source "$WT/.venv/bin/activate"
TAG=$1; REPS=$2; TH=$3; PS=$4; shift 4; mkdir -p logs
CPUS=$([ $TH = 1 ] && echo 0 || echo 0-3)
for r in $(seq 1 $REPS); do for p in ${PS//,/ }; do
  L=logs/${p}_t${TH}_${TAG}${r}.log
  grep -q '^RESULT' $L 2>/dev/null && continue
  taskset -c $CPUS python run1.py $p cand1114 1 $TH 60 $L "$@"
  echo "$L $(grep '^RESULT' $L)"
done; done

#!/bin/bash
# 一般の A/A: bash batch4.sh TAG MODE REPS THREADS 問題,... [K=V ...]
# MODE は n (普通) か s (遅い機械の真似: 同じ CPU に同じ数の `yes`)。ログ logs/<問題>_t<スレッド>_<TAG><回>.log。済んだものは飛ばす
cd "$(dirname "$0")"; WT=$(cd ../.. && pwd); source "$WT/.venv/bin/activate"
TAG=$1; MODE=$2; REPS=$3; TH=$4; PS=$5; shift 5; mkdir -p logs
CPUS=$([ $TH = 1 ] && echo 0 || echo 0-3)
for r in $(seq 1 $REPS); do for p in ${PS//,/ }; do
  L=logs/${p}_t${TH}_${TAG}${r}.log
  grep -q '^RESULT' $L 2>/dev/null && continue
  PIDS=()
  if [ $MODE = s ]; then for c in $(seq 0 $((TH-1))); do taskset -c $c yes >/dev/null & PIDS+=($!); done; fi
  taskset -c $CPUS python run1.py $p cand1114 1 $TH 60 $L "$@"
  for x in "${PIDS[@]}"; do kill $x; done; wait 2>/dev/null
  echo "$L $(grep '^RESULT' $L)"
done; done

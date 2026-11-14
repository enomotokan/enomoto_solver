#!/bin/bash
# A/A を回す: bash batch.sh MODE REPS (MODE は n (普通) か s (遅い機械の真似: ソルバーと同じ CPU に同じ数の空回し `yes` を載せ、速さを約半分にする))
# ログ logs/<問題>_t<スレッド>_<MODE><回>.log。済んだものは飛ばす (再開できる)。
cd "$(dirname "$0")"; WT=$(cd ../.. && pwd); source "$WT/.venv/bin/activate"
MODE=$1; REPS=$2; mkdir -p logs
for r in $(seq 1 $REPS); do
 for p in 10teams noswot nw04 mc11 physiciansched6-2; do
  for th in 1 4; do
   L=logs/${p}_t${th}_${MODE}${r}.log
   grep -q '^RESULT' $L 2>/dev/null && continue
   CPUS=$([ $th = 1 ] && echo 0 || echo 0-3)
   PIDS=()
   if [ $MODE = s ]; then for c in $(seq 0 $((th-1))); do taskset -c $c yes >/dev/null & PIDS+=($!); done; fi
   taskset -c $CPUS python run1.py $p cand1114 1 $th 60 $L
   for x in "${PIDS[@]}"; do kill $x; done; wait 2>/dev/null
   echo "$L $(grep '^RESULT' $L)"
  done
 done
done

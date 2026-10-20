#!/bin/bash
# fjcheck.sh <mps> <秒> <回数> [環境変数=値 ...]: ENOMOTO を回して FJ が解を見つけたか (found true) と最小の違反行数、最初の解を出す
mps=$1; t=$2; k=$3; shift 3
D=$(cd "$(dirname "$0")" && pwd)
for r in $(seq 1 $k); do
  out=$(env "$@" ENOMOTO_MIP_FJ_DEBUG=2 ENOMOTO_MIP_FJ_SEED=$r timeout $((t+60)) python -I $D/eno_run.py "$mps" $t 2>&1)
  fj=$(echo "$out" | grep "FJ2:" | head -1 | sed 's/.*least violated rows \([0-9]*\), found \([a-z]*\).*/viol=\1 found=\2/')
  pre=$(echo "$out" | grep "^Presolve:" | tail -1 | sed 's/.*-> //')
  root=$(echo "$out" | grep "^Root cuts" | sed 's/.*-> \([0-9.e+]*\).*/\1/')
  res=$(echo "$out" | grep RESULT | sed "s/.*'objective': \([^,]*\),.*'nodes': \([0-9]*\),.*'first_solution_time': \([^,]*\), 'first_solution_objective': \([^,]*\),.*/obj=\1 nodes=\2 first=\4@\3/")
  echo "run $r: pre=[$pre] fj1:[$fj] rootcuts=$root $res"
done

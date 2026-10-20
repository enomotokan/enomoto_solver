#!/bin/bash
# 試作が各問題で何をするかを洗い出す: screen.sh <問題...>
D=analysis/physiciansched6-2_20261020
for p in "$@"; do
  M=.miplib_cache/$p.mps
  [ -s $M ] || python scripts/run_miplib_benchmark.py --only $p --threads 4 --time-limit 1 --out /tmp/claude-0/screen_dl.json >/dev/null 2>&1
  out=$(ENOMOTO_MIP_CLIQUE_EXTEND=1 ENOMOTO_MIP_PROBE_EQUIV=1 ENOMOTO_MIP_MERGE_REPRESOLVE=1 ENOMOTO_MIP_INEQ_SINGLETON=1 ENOMOTO_DEBUG_INEQ_SINGLETON=1 ENOMOTO_MIP_LOG=1 timeout 120 python $D/eno_run.py $M 8 2>&1)
  ce=$(echo "$out" | grep -o "clique extension: [0-9]* rows extended, [0-9]* nonzeros added" | head -1 | sed 's/clique extension: //')
  ne=$(echo "$out" | grep -o "([0-9]* new equations)" | grep -o "[0-9]*" | paste -sd+ | bc)
  eq=$(echo "$out" | grep -o "[0-9]* equivalent binary pairs" | grep -o "^[0-9]*" | paste -sd+ | bc)
  is=$(echo "$out" | grep -o "DEBUG_INEQ_SINGLETON: fixes=[0-9]* implied_equalities=[0-9]*" | awk -F'[= ]' '{f+=$3; e+=$5} END {print f"/"e}')
  pre=$(echo "$out" | grep "^Presolve:" | head -1 | sed 's/Presolve: //')
  res=$(echo "$out" | grep RESULT | grep -o "'status': '[a-z_]*', 'objective': [^,]*")
  echo "$p | CE ${ce:-0} | newEq ${ne:-0} | equiv ${eq:-0} | ineqS fix/eq ${is} | $pre | $res"
done

#!/bin/bash
# side.sh: physiciansched6-2 の既定 (E2C なし) と、MIPLIB 10 問 (LP の反復が多い問題を含む) で既定・CUT_DSE_EXACT・ROWS_DSE_EXACT+LP_RESUME を比べる (60 秒・4 スレッド・各 1 回)
cd "$(dirname "$0")"
run() { p=$1; name=$2; shift 2; n=1; while [ -e logs/${p}_${name}_${n}.log ]; do n=$((n+1)); done
  env ENOMOTO_MIP_THREADS=4 "$@" python3 eno_run.py ../../.miplib_cache/$p.mps 60 4 > logs/${p}_${name}_${n}.log 2>&1
  echo "$p $name $n $(tail -1 logs/${p}_${name}_${n}.log)"; }
for cfg in "def" "cutexact ENOMOTO_MIP_CUT_DSE_EXACT=1" "rowsresume ENOMOTO_MIP_ROWS_DSE_EXACT=1 ENOMOTO_MIP_LP_RESUME=1"; do
  set -- $cfg; name=$1; shift
  run physiciansched6-2 $name ENOMOTO_MIP_LOG=1 "$@"
done
run physiciansched6-2 e2c_all ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_EQUIV2_CUTS=1 ENOMOTO_MIP_CUT_DSE_EXACT=1 ENOMOTO_MIP_ROWS_DSE_EXACT=1 ENOMOTO_MIP_LP_RESUME=1
for p in 10teams qnet1 misc07 neos-1456979 nw04 mik-250-20-75-4 neos-911970 binkar10_1 neos5 air03; do
  for cfg in "def" "cutexact ENOMOTO_MIP_CUT_DSE_EXACT=1" "rowsresume ENOMOTO_MIP_ROWS_DSE_EXACT=1 ENOMOTO_MIP_LP_RESUME=1"; do
    set -- $cfg; name=$1; shift
    run $p $name "$@"
  done
done

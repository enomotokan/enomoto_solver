#!/bin/bash
# 試作 ENOMOTO_MIP_SYMAVG の誤答の確かめ。リポジトリの根から: bash analysis/symavg_20261121/fuzz.sh → fuzz/<名前>.txt・.cov (平均したカットを作った求解の数)
set -u
cd "$(dirname "$0")/../.."
F=analysis/symavg_20261121/fuzz; mkdir -p $F
run() { name=$1; shift; env ENOMOTO_MIP_LOG=1 "$@" 2> $F/$name.err | grep -E "^(seed|bad)" | tail -20 > $F/$name.txt; echo "$(grep -c 'SYMAVG: [1-9]' $F/$name.err) solves made averaged cuts / $(grep -c 'SYMAVG:' $F/$name.err) solves with the generators" > $F/$name.cov; rm -f $F/$name.err; echo "$name: $(tail -1 $F/$name.txt) / $(cat $F/$name.cov)"; }
run sym1_avg1 env ENOMOTO_MIP_SYMAVG=1 python3 scripts/mip_fuzz_symmetric.py 300 0 1
run sym2_avg1 env ENOMOTO_MIP_SYMAVG=1 python3 scripts/mip_fuzz_symmetric.py 300 0 2
run sym2_noorb_avg1 env ENOMOTO_MIP_SYMAVG=1 ENOMOTO_MIP_NO_ORBITOPE=1 python3 scripts/mip_fuzz_symmetric.py 300 0 2
run sym2_noorb_avgv env ENOMOTO_MIP_SYMAVG=v ENOMOTO_MIP_NO_ORBITOPE=1 python3 scripts/mip_fuzz_symmetric.py 300 300 2
run sym2_noorb_avg1_t1 env ENOMOTO_MIP_SYMAVG=1 ENOMOTO_MIP_NO_ORBITOPE=1 ENOMOTO_MIP_THREADS=1 python3 scripts/mip_fuzz_symmetric.py 300 600 2
run pplex0_avg1 env ENOMOTO_MIP_SYMAVG=1 ENOMOTO_MIP_SYM_PPLEX=0 python3 analysis/scipsym_20261120/fuzz_pplex.py 300 11000
run vs_highs_avg1 env ENOMOTO_MIP_SYMAVG=1 ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300 4000
